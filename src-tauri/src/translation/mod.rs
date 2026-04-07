pub mod commands;

use anyhow::{anyhow, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use tokenizers::Tokenizer;

static ENCODER_BYTES: &[u8] = include_bytes!("assets/encoder_model_quantized.onnx");
static DECODER_BYTES: &[u8] = include_bytes!("assets/decoder_model_quantized.onnx");
static TOKENIZER_BYTES: &[u8] = include_bytes!("assets/tokenizer.json");

pub struct Translator {
    encoder: Session,
    decoder: Session,
    tokenizer: Tokenizer,
}

impl Translator {
    pub fn new() -> Result<Self> {
        let encoder = Session::builder()
            .map_err(|e| anyhow!("Builder error: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("Opt error: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow!("Thread error: {}", e))?
            .commit_from_memory(ENCODER_BYTES)
            .map_err(|e| anyhow!("Encoder load error: {}", e))?;

        let decoder = Session::builder()
            .map_err(|e| anyhow!("Builder error: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow!("Opt error: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow!("Thread error: {}", e))?
            .commit_from_memory(DECODER_BYTES)
            .map_err(|e| anyhow!("Decoder load error: {}", e))?;

        let tokenizer = Tokenizer::from_bytes(TOKENIZER_BYTES)
            .map_err(|e| anyhow!("Tokenizer error: {}", e))?;

        Ok(Self {
            encoder,
            decoder,
            tokenizer,
        })
    }

    pub fn translate(&mut self, text: &str) -> Result<String> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow!("Encode error: {}", e))?;

        let mut input_ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();

        input_ids.push(0);

        let attention_mask: Vec<i64> = vec![1; input_ids.len()];
        let seq_len = input_ids.len();

        let input_shape = vec![1, seq_len];
        let input_ids_val = Value::from_array((input_shape.clone(), input_ids.clone()))
            .map_err(|e| anyhow!("Value error: {}", e))?
            .into_dyn();
        let attention_mask_val = Value::from_array((input_shape.clone(), attention_mask.clone()))
            .map_err(|e| anyhow!("Value error: {}", e))?
            .into_dyn();

        let encoder_inputs = vec![
            ("input_ids", input_ids_val),
            ("attention_mask", attention_mask_val),
        ];

        let encoder_outputs = self
            .encoder
            .run(encoder_inputs)
            .map_err(|e| anyhow!("Encoder run error: {}", e))?;

        let hidden_val = encoder_outputs
            .get("last_hidden_state")
            .ok_or_else(|| anyhow!("Missing hidden state"))?;

        let (_, hidden_data) = hidden_val.try_extract_tensor::<f32>()?;
        let hidden_vec: Vec<f32> = hidden_data.to_vec();
        let hidden_dim = hidden_data.len() / seq_len;

        let mut generated_ids = vec![62517i64];
        let mut result_text = String::new();
        let eos_token_id = 0u32; // </s>

        for _step in 0..128 {
            let cur_len = generated_ids.len();
            let dec_shape = vec![1, cur_len];
            let dec_val = Value::from_array((dec_shape, generated_ids.clone()))
                .map_err(|e| anyhow!("Dec value error: {}", e))?
                .into_dyn();

            let enc_shape = vec![1, seq_len, hidden_dim];
            let enc_val = Value::from_array((enc_shape, hidden_vec.clone()))
                .map_err(|e| anyhow!("Enc value error: {}", e))?
                .into_dyn();

            let enc_mask_val = Value::from_array((input_shape.clone(), attention_mask.clone()))
                .map_err(|e| anyhow!("Mask value error: {}", e))?
                .into_dyn();

            let decoder_inputs = vec![
                ("input_ids", dec_val),
                ("encoder_hidden_states", enc_val),
                ("encoder_attention_mask", enc_mask_val),
            ];

            let decoder_outputs = self
                .decoder
                .run(decoder_inputs)
                .map_err(|e| anyhow!("Decoder run error: {}", e))?;

            let logits_val = decoder_outputs
                .get("logits")
                .ok_or_else(|| anyhow!("Missing logits"));

            let (_, logits_data) = logits_val?.try_extract_tensor::<f32>()?;

            let vocab_size = logits_data.len() / cur_len;
            let start_idx = (cur_len - 1) * vocab_size;

            let mut max_idx = 0;
            let mut max_val = logits_data[start_idx];
            for i in 1..vocab_size {
                let idx = start_idx + i;
                if idx < logits_data.len() && logits_data[idx] > max_val {
                    max_val = logits_data[idx];
                    max_idx = i;
                }
            }

            let next_token_id = max_idx as u32;

            if next_token_id == eos_token_id {
                break;
            }

            let token = self
                .tokenizer
                .id_to_token(next_token_id)
                .unwrap_or_default();
            let token = token.replace('▁', " ");

            result_text.push_str(&token);
            generated_ids.push(next_token_id as i64);
        }

        Ok(result_text.trim().to_string())
    }
}
