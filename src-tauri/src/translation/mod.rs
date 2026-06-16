pub mod commands;

use anyhow::{anyhow, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const MODEL_FILENAME: &str = "Hy-MT2-1.8B-Q4_0.gguf";

static INSTANCE: OnceLock<Arc<Mutex<Translator>>> = OnceLock::new();

/// Lazy-init on first translation request.
pub fn get_translator(app_data: &PathBuf) -> Result<Arc<Mutex<Translator>>> {
    if let Some(t) = INSTANCE.get() {
        return Ok(Arc::clone(t));
    }
    let model_path = ensure_model(app_data)?;
    let translator = Translator::load(&model_path)?;
    let arc = Arc::new(Mutex::new(translator));
    let stored = INSTANCE.get_or_init(|| arc);
    Ok(Arc::clone(stored))
}

fn ensure_model(app_data: &PathBuf) -> Result<PathBuf> {
    let models_dir = app_data.join("models");
    std::fs::create_dir_all(&models_dir)?;
    let model_path = models_dir.join(MODEL_FILENAME);
    if !model_path.exists() {
        return Err(anyhow!(
            "Model file not found at {}. Call ensure_model_bytes first.",
            model_path.display()
        ));
    }
    Ok(model_path)
}

pub struct Translator {
    model: LlamaModel,
    backend: LlamaBackend,
}

impl Translator {
    fn load(model_path: &PathBuf) -> Result<Self> {
        // Workaround for Adreno Vulkan GPU bugs:
        // Setting GGML_VK_DISABLE_F16=1 forces the Vulkan backend to use FP32
        // arithmetic instead of FP16. On Adreno GPUs, FP16 can produce garbled
        // / repeated token output with Q4_0 models. This env var must be set
        // before LlamaBackend::init() to take effect.
        std::env::set_var("GGML_VK_DISABLE_F16", "1");

        let backend = LlamaBackend::init()
            .map_err(|e| anyhow!("llama backend init: {}", e))?;
        let path_str = model_path.to_str()
            .ok_or_else(|| anyhow!("non-utf8 model path"))?;
        let model = LlamaModel::load_from_file(&backend, path_str, &LlamaModelParams::default())
            .map_err(|e| anyhow!("Failed to load translation model: {}", e))?;

        // Run a minimal warmup decode to pre-compile Vulkan compute shaders.
        // On Adreno GPUs, the first pipeline creation (especially
        // mul_mat_vec_q4_0_f32_f32) can take >20s of synchronous compilation.
        // Paying that cost here during load prevents a long hang on the first
        // user translation.
        if let Err(e) = Self::warmup_vulkan_pipelines(&model, &backend) {
            eprintln!("[translation] Vulkan warmup failed (non-fatal): {}", e);
        }

        Ok(Self { model, backend })
    }

    /// Trigger Vulkan pipeline compilation by running a trivial decode.
    /// This is discarded immediately — its only purpose is to force lazy
    /// shader compilation before the user asks for a real translation.
    fn warmup_vulkan_pipelines(model: &LlamaModel, backend: &LlamaBackend) -> Result<()> {
        let n_threads = num_cpus::get_physical() as i32;
        let mut ctx = model.new_context(
            backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(32))
                .with_n_threads(n_threads)
                .with_n_threads_batch(n_threads),
        )?;

        let warmup_text = "Hello";
        let tokens = model.str_to_token(warmup_text, AddBos::Never)?;
        let mut batch = LlamaBatch::new(tokens.len() + 1, 1);
        batch.add_sequence(&tokens, 0, false)?;
        ctx.decode(&mut batch)?;

        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
        let token = sampler.sample(&ctx, -1);
        sampler.accept(token);
        if !model.is_eog_token(token) {
            batch.clear();
            batch.add(token, tokens.len() as i32, &[0], true)?;
            ctx.decode(&mut batch)?;
        }

        drop(ctx);
        Ok(())
    }

    pub fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String> {
        let source_name = language_name(source_lang);
        let target_name = language_name(target_lang);

        let n_threads = num_cpus::get_physical() as i32;
        let mut ctx = self.model.new_context(
            &self.backend,
            LlamaContextParams::default()
                .with_n_ctx(NonZeroU32::new(512))
                .with_n_threads(n_threads)
                .with_n_threads_batch(n_threads)
                .with_n_batch(256)
                .with_n_ubatch(8),
        )?;

        let tmpl = self.model.chat_template(None)?;

        let system = if source_lang == "AUTO" {
            format!("You are a translator. Translate the user's message to {}. Output ONLY the translation, no additional text.", target_name)
        } else {
            format!("You are a translator. Translate the user's message from {} to {}. Output ONLY the translation, no additional text.", source_name, target_name)
        };

        let messages = [
            LlamaChatMessage::new("system".to_string(), system)?,
            LlamaChatMessage::new("user".to_string(), text.to_string())?,
        ];
        let prompt = self.model.apply_chat_template(&tmpl, &messages, true)?;

        let tokens = self.model.str_to_token(&prompt, AddBos::Never)?;
        let n_prompt = tokens.len();
        let mut batch = LlamaBatch::new(n_prompt + 256, 1);
        batch.add_sequence(&tokens, 0, false)?;
        ctx.decode(&mut batch)?;

        let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
        let mut generated: Vec<llama_cpp_2::token::LlamaToken> = Vec::new();

        for _ in 0..256 {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);
            if self.model.is_eog_token(token) { break; }
            batch.clear();
            batch.add(token, (n_prompt + generated.len()) as i32, &[0], true)?;
            ctx.decode(&mut batch)?;
            generated.push(token);
        }

        let mut translation = String::new();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        for token in &generated {
            translation.push_str(&self.model.token_to_piece(*token, &mut decoder, false, None)?);
        }
        Ok(translation.trim().to_string())
    }
}

pub fn language_name(code: &str) -> &str {
    match code {
        "AR" => "Arabic", "BG" => "Bulgarian", "ZH" => "Chinese (Simplified)",
        "ZH-TR" => "Chinese (Traditional)", "HR" => "Croatian", "CS" => "Czech",
        "DA" => "Danish", "NL" => "Dutch", "EN" => "English", "ET" => "Estonian",
        "FI" => "Finnish", "FR" => "French", "DE" => "German", "EL" => "Greek",
        "HE" => "Hebrew", "HI" => "Hindi", "HU" => "Hungarian", "ID" => "Indonesian",
        "IT" => "Italian", "JA" => "Japanese", "KO" => "Korean", "LV" => "Latvian",
        "LT" => "Lithuanian", "NO" => "Norwegian", "FA" => "Persian", "PL" => "Polish",
        "PT" => "Portuguese", "RO" => "Romanian", "RU" => "Russian", "SR" => "Serbian",
        "SK" => "Slovak", "SL" => "Slovenian", "ES" => "Spanish", "SV" => "Swedish",
        "TH" => "Thai", "TR" => "Turkish", "UK" => "Ukrainian", "VI" => "Vietnamese",
        _ => code,
    }
}
