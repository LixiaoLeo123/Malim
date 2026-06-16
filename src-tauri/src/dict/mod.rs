use flate2::read::GzDecoder;
use regex::Regex;
use rsmorphy::opencorpora::Dictionary;
use rsmorphy::MorphAnalyzer;
use rsmorphy::Source;
use rust_mdict::{KeyWordItem, Mdd, Mdx};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub headword: String,
    pub lemma: String,
    pub forms: Vec<String>,
    pub definition_html: String,
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionarySearchResponse {
    pub query: String,
    pub normalized_query: String,
    pub results: Vec<DictionaryEntry>,
}

// ── Russian service (morphology-aware MDX) ────────────────────

struct RussianDictService {
    mdx: Mdx,
    _mdd: Option<Mdd>,
    morph: MorphAnalyzer,
}

// ── Rich TSV service (Korean / Spanish) ───────────────────────

struct TsvDictService {
    entries: Vec<(String, String)>, // sorted (headword, html_definition)
    lang: String,
}

static RUSSIAN_SERVICE: OnceLock<Result<Mutex<RussianDictService>, String>> = OnceLock::new();
static KOREAN_SERVICE: OnceLock<Result<Mutex<TsvDictService>, String>> = OnceLock::new();
static SPANISH_SERVICE: OnceLock<Result<Mutex<TsvDictService>, String>> = OnceLock::new();

const EMBEDDED_RU_MDX: &[u8] = include_bytes!("assets/OpenRussian.mdx");
const EMBEDDED_RU_MDD: &[u8] = include_bytes!("assets/OpenRussian.mdd");
const EMBEDDED_KO_GZ: &[u8] = include_bytes!("assets/korean_en.txt.gz");
const EMBEDDED_ES_GZ: &[u8] = include_bytes!("assets/spanish_en.txt.gz");

// ═══════════════════════════════════════════ Commands

#[tauri::command]
pub async fn preload_russian_dictionary(app: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || { let _ = russian_service(&app); }).await.map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn preload_korean_dictionary(app: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || { let _ = korean_service(&app); }).await.map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn preload_spanish_dictionary(app: AppHandle) -> Result<(), String> {
    tokio::task::spawn_blocking(move || { let _ = spanish_service(&app); }).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_russian_dictionary(app: AppHandle, query: String) -> Result<DictionarySearchResponse, String> {
    let q = query.clone();
    tokio::task::spawn_blocking(move || search_russian_blocking(&app, &q))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r)
}
#[tauri::command]
pub async fn search_korean_dictionary(app: AppHandle, query: String) -> Result<DictionarySearchResponse, String> {
    let q = query.clone();
    tokio::task::spawn_blocking(move || search_tsv_blocking(&app, &q, &korean_service))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r)
}
#[tauri::command]
pub async fn search_spanish_dictionary(app: AppHandle, query: String) -> Result<DictionarySearchResponse, String> {
    let q = query.clone();
    tokio::task::spawn_blocking(move || search_tsv_blocking(&app, &q, &spanish_service))
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r)
}

// ═══════════════════════════════════════════ Russian

fn russian_service(app: &AppHandle) -> Result<&'static Mutex<RussianDictService>, String> {
    RUSSIAN_SERVICE.get_or_init(|| RussianDictService::new(app).map(Mutex::new)).as_ref().map_err(|e| e.clone())
}

impl RussianDictService {
    fn new(app: &AppHandle) -> Result<Self, String> {
        let (mdx_path, mdd_path) = ensure_russian_files(app)?;
        let morph = load_russian_morphology(app)?;
        let mdx = Mdx::new(&mdx_path).map_err(|e| e.to_string())?;
        let mdd = Mdd::new(&mdd_path).map(Some).unwrap_or_else(|e| { eprintln!("OpenRussian.mdd: {e}"); None });
        Ok(Self { mdx, _mdd: mdd, morph })
    }
}
fn ensure_russian_files(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let dir = dict_dir(app, "openrussian_dictionary")?;
    let mdx = dir.join("OpenRussian.mdx");
    let mdd = dir.join("OpenRussian.mdd");
    if !mdx.exists() { std::fs::write(&mdx, EMBEDDED_RU_MDX).map_err(|e| e.to_string())?; }
    if !mdd.exists() { std::fs::write(&mdd, EMBEDDED_RU_MDD).map_err(|e| e.to_string())?; }
    Ok((mdx, mdd))
}
fn load_russian_morphology(app: &AppHandle) -> Result<MorphAnalyzer, String> {
    let p = crate::memory::ensure_dict_files(app);
    Ok(MorphAnalyzer::new(Dictionary::from_file(p)))
}

fn search_russian_blocking(app: &AppHandle, query: &str) -> Result<DictionarySearchResponse, String> {
    let cleaned = sanitize_query(query);
    let nq = normalize_lookup_key(&cleaned);
    if nq.is_empty() { return Ok(empty_resp(query, &nq)); }
    let svc = russian_service(app)?;
    let mut svc = svc.lock().map_err(|_| "mutex")?;
    let lem = cleaned.clone();
    if normalize_lookup_key(&lem).is_empty() { return Ok(empty_resp(query, &nq)); }
    let mut terms = vec![lem.clone()];
    if normalize_lookup_key(&cleaned) != nq { terms.push(cleaned.clone()); }
    for c in build_lookup_candidates_ru(&svc.morph, &lem) {
        if normalize_lookup_key(&c) != nq { terms.push(c); }
    }
    let mut raw = Vec::new();
    let mut seen = BTreeSet::new();
    for term in terms {
        let nc = normalize_lookup_key(&term);
        if nc.is_empty() { continue; }
        let Some((start, end)) = find_exact_range(svc.mdx.keyword_list(), &nc)? else { continue; };
        for idx in start..end {
            let ki = svc.mdx.keyword_list().get(idx).cloned().ok_or_else(|| format!("idx {idx}"))?;
            if !seen.insert(format!("{}|{}", term, ki.key_text)) { continue; }
            raw.push(build_ru_entry(&mut svc, &ki, &term)?);
        }
    }
    Ok(DictionarySearchResponse { query: query.to_string(), normalized_query: nq, results: merge_entries(raw) })
}

fn build_ru_entry(svc: &mut RussianDictService, item: &KeyWordItem, matched_term: &str) -> Result<DictionaryEntry, String> {
    let lookup = svc.mdx.fetch(item).ok_or_else(|| format!("missing {}", item.key_text))?;
    let hw = lookup.key_text.to_string();
    let def = cleanup_html(&lookup.definition);
    let (lemma, forms) = expand_lexeme_ru(&svc.morph, &hw);
    Ok(DictionaryEntry { headword: hw, lemma, forms, definition_html: def, matched_terms: vec![matched_term.to_string()] })
}

fn build_lookup_candidates_ru(morph: &MorphAnalyzer, query: &str) -> Vec<String> {
    let mut c = BTreeSet::new();
    let cl = sanitize_query(query);
    if !cl.is_empty() { c.insert(cl.clone()); }
    if cl.contains('ё') { c.insert(cl.replace('ё', "е")); }
    if cl.contains('е') { c.insert(cl.replace('е', "ё")); }
    match catch_unwind(AssertUnwindSafe(|| morph.parse(&cl))) {
        Ok(parsed) => { for p in parsed.into_iter().take(8) {
            let l = sanitize_query(&p.lex.get_lemma(morph).get_word()); if !l.is_empty() { c.insert(l); }
            let nf = sanitize_query(&p.lex.get_normal_form(morph)); if !nf.is_empty() { c.insert(nf); }
        }}
        Err(_) => eprintln!("rsmorphy: {cl}"),
    }
    c.into_iter().collect()
}
fn expand_lexeme_ru(morph: &MorphAnalyzer, headword: &str) -> (String, Vec<String>) {
    let mut lemma = sanitize_query(headword); let mut forms = BTreeSet::new();
    match catch_unwind(AssertUnwindSafe(|| morph.parse(headword))) {
        Ok(parsed) => { for p in parsed.into_iter().take(8) {
            lemma = sanitize_query(&p.lex.get_lemma(morph).get_word());
            for lex in p.lex.get_lexeme(morph) { let f = sanitize_query(&lex.get_word()); if !f.is_empty() { forms.insert(f); } }
        }}
        Err(_) => eprintln!("rsmorphy lexeme: {headword}"),
    }
    if forms.is_empty() { forms.insert(lemma.clone()); }
    if lemma.is_empty() { lemma = sanitize_query(headword); }
    (lemma, forms.into_iter().collect())
}

// ═══════════════════════════════════════════ TSV dicts (Korean / Spanish)

fn korean_service(app: &AppHandle) -> Result<&'static Mutex<TsvDictService>, String> {
    KOREAN_SERVICE.get_or_init(|| TsvDictService::new(app, "korean", EMBEDDED_KO_GZ).map(Mutex::new)).as_ref().map_err(|e| e.clone())
}
fn spanish_service(app: &AppHandle) -> Result<&'static Mutex<TsvDictService>, String> {
    SPANISH_SERVICE.get_or_init(|| TsvDictService::new(app, "spanish", EMBEDDED_ES_GZ).map(Mutex::new)).as_ref().map_err(|e| e.clone())
}

impl TsvDictService {
    fn new(app: &AppHandle, lang: &str, embedded_gz: &[u8]) -> Result<Self, String> {
        let dir = dict_dir(app, &format!("{}_dictionary", lang))?;
        let path = dir.join(format!("{}_en.txt", lang));

        // Write the decompressed text for persistence
        if !path.exists() {
            let mut decoder = GzDecoder::new(embedded_gz);
            let mut text = String::new();
            decoder.read_to_string(&mut text).map_err(|e| format!("decompress {lang}: {e}"))?;
            std::fs::write(&path, &text).map_err(|e| format!("write {lang}: {e}"))?;
        }

        let data = std::fs::read_to_string(&path).map_err(|e| format!("read {lang}: {e}"))?;
        let mut entries: Vec<(String, String)> = Vec::new();

        // Korean dictionary has multi-line entries: each new entry starts with
        // a line containing \t, continuation lines are pure HTML (no tab).
        // Spanish dictionary has single-line entries: word\t<d>...</d> per line.
        // The multi-line parser handles both formats correctly.
        let mut current_word: Option<String> = None;
        let mut current_def = String::new();
        for line in data.lines() {
            if let Some((word, def)) = line.split_once('\t') {
                // Flush previous entry
                if let Some(w) = current_word.take() {
                    if !w.is_empty() {
                        entries.push((w, std::mem::take(&mut current_def)));
                    }
                }
                if !word.is_empty() {
                    current_word = Some(word.to_string());
                    current_def = def.to_string();
                }
            } else {
                // Continuation line for current entry
                if current_word.is_some() {
                    if !current_def.is_empty() {
                        current_def.push('\n');
                    }
                    current_def.push_str(line);
                }
            }
        }
        // Flush the last entry
        if let Some(w) = current_word {
            if !w.is_empty() {
                entries.push((w, current_def));
            }
        }
        // Transform Spanish HTML to mdx-entry format (compile regexes once)
        if lang == "spanish" {
            let re_c = Regex::new(r"<c>([^<]*)</c>").unwrap();
            let re_ipa = Regex::new(r"<i>//([^/]*)//</i>").unwrap();
            let re_ipa2 = Regex::new(r"<i>/([^/]+)/</i>").unwrap();
            let re_fn = Regex::new(r"<i>\(([^)]*)\)</i>").unwrap();
            let re_rem_i = Regex::new(r"</?i>").unwrap();
            for (_, def) in entries.iter_mut() {
                *def = transform_spanish_html(def, &re_c, &re_ipa, &re_ipa2, &re_fn, &re_rem_i);
            }
        }
        entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        entries.dedup_by(|a, b| a.0.to_lowercase() == b.0.to_lowercase());
        Ok(Self { entries, lang: lang.to_string() })
    }

    fn search(&self, query: &str) -> Vec<DictionaryEntry> {
        let q = query.trim().to_lowercase();
        if q.is_empty() { return vec![]; }

        // Stage 1: Exact match
        let mut results = self.exact_search(&q);
        if !results.is_empty() { return results; }

        // Stage 2: Prefix match
        results = self.prefix_search(&q, 15);
        if !results.is_empty() { return results; }

        // Stage 3: Accent-insensitive retry (Spanish only)
        if self.lang == "spanish" {
            let no_accent: String = q.chars().map(|c| match c {
                'á' => 'a', 'é' => 'e', 'í' => 'i', 'ó' => 'o', 'ú' => 'u', 'ü' => 'u', 'ñ' => 'n',
                _ => c,
            }).collect();
            if no_accent != q {
                results = self.exact_search(&no_accent);
                if !results.is_empty() { return results; }
                results = self.prefix_search(&no_accent, 15);
                if !results.is_empty() { return results; }
            }
        }

        // Stage 4: Candidate generation
        let candidates = if self.lang == "spanish" {
            generate_spanish_candidates(&q)
        } else if self.lang == "korean" {
            generate_korean_candidates(&q)
        } else {
            vec![]
        };
        for cand in &candidates {
            let nc = cand.to_lowercase();
            results = self.exact_search(&nc);
            if !results.is_empty() { return results; }
            results = self.prefix_search(&nc, 10);
            if !results.is_empty() { return results; }
        }

        // Stage 5: Substring fallback
        substring_search(&self.entries, &q, 20)
    }

    fn exact_search(&self, q: &str) -> Vec<DictionaryEntry> {
        let mut results = Vec::new();
        let ql = q.to_string();
        match self.entries.binary_search_by(|(w, _)| w.to_lowercase().cmp(&ql)) {
            Ok(pos) => {
                let mut i = pos;
                while i > 0 && self.entries[i-1].0.to_lowercase() == ql { i -= 1; }
                while i < self.entries.len() && self.entries[i].0.to_lowercase() == ql {
                    let (w, d) = &self.entries[i];
                    results.push(DictionaryEntry { headword: w.clone(), lemma: w.clone(), forms: vec![], definition_html: d.clone(), matched_terms: vec![ql.clone()] });
                    i += 1;
                }
            }
            Err(_) => {}
        }
        results
    }

    fn prefix_search(&self, q: &str, max: usize) -> Vec<DictionaryEntry> {
        let mut results = Vec::new();
        let ql = q.to_string();
        match self.entries.binary_search_by(|(w, _)| w.to_lowercase().cmp(&ql)) {
            Err(ins) => {
                for i in ins..self.entries.len() {
                    if !self.entries[i].0.to_lowercase().starts_with(q) { break; }
                    if results.len() >= max { break; }
                    let (w, d) = &self.entries[i];
                    results.push(DictionaryEntry { headword: w.clone(), lemma: w.clone(), forms: vec![], definition_html: d.clone(), matched_terms: vec![ql.clone()] });
                }
            }
            _ => {} // exact match already handled
        }
        results
    }
}

// ── Spanish HTML → mdx-entry transformation ──────────────────

fn transform_spanish_html(html: &str, re_c: &Regex, re_ipa: &Regex, re_ipa2: &Regex, re_fn: &Regex, re_rem_i: &Regex) -> String {
    let mut s = html.to_string();

    // Wrap in mdx-entry div: replace <d> / </d>
    s = s.replace("<d>", "<div class=\"mdx-entry\">").replace("</d>", "</div>");

    // Word class tag: <c>pos</c> → <span class="pos">pos</span>
    s = re_c.replace_all(&s, r#"<span class="pos">$1</span>"#).to_string();

    // IPA (double slash): <i>//text//</i> → <span class="ipa">[text]</span>
    s = re_ipa.replace_all(&s, r#"<span class="ipa">[$1]</span>"#).to_string();

    // IPA (single slash): <i>/text/</i> → <span class="ipa">[text]</span>
    s = re_ipa2.replace_all(&s, r#"<span class="ipa">[$1]</span>"#).to_string();

    // Form notes: <i>(text)</i> → <em>text</em>
    s = re_fn.replace_all(&s, r"<em>$1</em>").to_string();

    // Meanings section: <m> → <div class="section"><h2>Definitions</h2>, </m> → </div>
    s = s.replace("<m>", "<div class=\"section\"><h2>Definitions</h2>")
         .replace("</m>", "</div>");

    // Unordered list: <u> → <ul>, </u> → </ul>
    s = s.replace("<u>", "<ul>").replace("</u>", "</ul>");

    // Forms section: <f> → wrapper, </f> → close
    s = s.replace("<f>", "<div class=\"section\"><h2>Forms</h2><div class=\"forms-grid\">")
         .replace("</f>", "</div></div>");

    // Sub-form items: <s> → <span class="form-item">, </s> → </span>
    s = s.replace("<s>", "<span class=\"form-item\">").replace("</s>", "</span>");

    // HR separator
    s = s.replace("<hr/>", "<hr class=\"mdx-sep\" />");

    // Clean up any remaining raw <i> tags (unmatched by IPA/form-note patterns)
    re_rem_i.replace_all(&s, "").to_string()
}

// ── Candidate generation for fallback search ──────────────────

fn generate_spanish_candidates(query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    let mut candidates = BTreeSet::new();

    // 1. Accent stripping
    let no_accent: String = q.chars().map(|c| match c {
        'á' => 'a', 'é' => 'e', 'í' => 'i', 'ó' => 'o', 'ú' => 'u', 'ü' => 'u', 'ñ' => 'n',
        _ => c,
    }).collect();
    if no_accent != q { candidates.insert(no_accent.clone()); }

    // 2. Plural → singular: remove trailing -s, -es, -as, -os
    if q.ends_with("as") && q.len() > 3 {
        let stem = &q[..q.len()-2];
        candidates.insert(stem.to_string());           // buenas → buena
        candidates.insert(format!("{}o", stem));       // buenas → bueno (masc sg)
    } else if q.ends_with("os") && q.len() > 3 {
        candidates.insert(q[..q.len()-2].to_string()); // gatos → gato
        candidates.insert(q[..q.len()-1].to_string()); // gatos → gato (alt strip -s)
    } else if q.ends_with("es") && q.len() > 3 {
        let stem = &q[..q.len()-2];
        candidates.insert(stem.to_string());           // grandes → grande
        candidates.insert(format!("{}o", stem));       // grandes → grando
    } else if q.ends_with('s') && q.len() > 2 {
        candidates.insert(q[..q.len()-1].to_string()); // casas → casa
        candidates.insert(format!("{}o", &q[..q.len()-1])); // casas → caso
    }

    // 3. Feminine → masculine: -a → -o
    if q.ends_with('a') && q.len() > 2 {
        let stem = &q[..q.len()-1];
        candidates.insert(format!("{}o", stem));
        // Also try stripping -a and -s for plural feminine forms
        candidates.insert(stem.to_string());           // -a stripped
    }

    // 4. Verb gerund: -ando/-iendo → infinitive
    if q.ends_with("ando") && q.len() > 5 {
        let stem = &q[..q.len()-4];
        candidates.insert(format!("{}ar", stem));
    }
    if q.ends_with("iendo") && q.len() > 6 {
        let stem = &q[..q.len()-5];
        candidates.insert(format!("{}er", stem));
        candidates.insert(format!("{}ir", stem));
    }

    // 5. Past participle: -ado/-ido → infinitive
    if q.ends_with("ado") && q.len() > 4 {
        let stem = &q[..q.len()-3];
        candidates.insert(format!("{}ar", stem));
    }
    if q.ends_with("ido") && q.len() > 4 {
        let stem = &q[..q.len()-3];
        candidates.insert(format!("{}er", stem));
        candidates.insert(format!("{}ir", stem));
    }

    // 6. Common present indicative: -o, -as, -a, -amos, -an → -ar
    //    -o, -es, -e, -emos, -en → -er; -o, -es, -e, -imos, -en → -ir
    let verb_endings: &[(&str, &[&str])] = &[
        ("ar", &["o", "as", "a", "amos", "áis", "an",
                   "aba", "abas", "ábamos", "abais", "aban",
                   "é", "aste", "ó", "amos", "asteis", "aron",
                   "aré", "arás", "ará", "aremos", "aréis", "arán"]),
        ("er", &["o", "es", "e", "emos", "éis", "en",
                   "ía", "ías", "íamos", "íais", "ían",
                   "í", "iste", "ió", "imos", "isteis", "ieron"]),
        ("ir", &["o", "es", "e", "imos", "ís", "en",
                   "ía", "ías", "íamos", "íais", "ían",
                   "í", "iste", "ió", "imos", "isteis", "ieron"]),
    ];
    for &(inf_suffix, endings) in verb_endings {
        for &ending in endings {
            if q.ends_with(ending) && q.len() > ending.len() + 1 {
                let stem = &q[..q.len()-ending.len()];
                candidates.insert(format!("{}{}", stem, inf_suffix));
            }
        }
    }

    // 7. Also try common subjunctive endings
    let subj_endings = &["e", "es", "emos", "éis", "en", "a", "as", "amos", "áis", "an"];
    for &ending in subj_endings {
        if q.ends_with(ending) && q.len() > ending.len() + 1 {
            let stem = &q[..q.len()-ending.len()];
            candidates.insert(format!("{}ar", stem));
            candidates.insert(format!("{}er", stem));
            candidates.insert(format!("{}ir", stem));
        }
    }

    // Dedup and limit
    candidates.into_iter()
        .filter(|c| c.len() >= 2 && c != &q)
        .take(20)
        .collect()
}

fn generate_korean_candidates(query: &str) -> Vec<String> {
    let q = query.trim().to_string();
    let mut candidates = BTreeSet::new();

    // Helper: drop last N chars safely (works with multi-byte UTF-8)
    fn drop_last_n(s: &str, n: usize) -> String {
        let char_count = s.chars().count();
        if char_count <= n { return String::new(); }
        s.chars().take(char_count - n).collect()
    }

    // 1. Strip common postpositions (ordered by length, longest first)
    let postpositions: &[&str] = &[
        "에서", "에게", "으로", "부터", "까지", "보다", "처럼", "만큼", "조차", "마저",
        "은", "는", "이", "가", "을", "를", "에", "도", "만", "로", "과", "와", "의",
    ];
    for &pp in postpositions {
        if q.ends_with(pp) && q.chars().count() > pp.chars().count() {
            candidates.insert(drop_last_n(&q, pp.chars().count()));
            break; // only strip outermost
        }
    }

    // 2. Strip verb endings and try dictionary form
    // Polite present: -ㅂ니다 / -습니다
    if q.ends_with("습니다") {
        candidates.insert(format!("{}다", drop_last_n(&q, 3))); // drop "니다"
    } else if q.ends_with("ㅂ니다") {
        candidates.insert(format!("{}다", drop_last_n(&q, 3))); // drop "니다"
    }

    // Polite informal: -어요/-아요/-여요 (3-char suffixes)
    for &suffix in &["어요", "아요", "여요"] {
        if q.ends_with(suffix) && q.chars().count() > suffix.chars().count() {
            let stem = drop_last_n(&q, suffix.chars().count());
            if !stem.is_empty() {
                candidates.insert(format!("{}다", stem));
                candidates.insert(stem);
            }
        }
    }

    // General "-요" ending (covers contracted forms like 가요, 봐요, etc.)
    if q.ends_with("요") && q.chars().count() > 1 {
        let stem = drop_last_n(&q, 1);
        if !stem.is_empty() {
            candidates.insert(format!("{}다", stem));
        }
    }

    // Casual informal: -어/-아/-여 (non-polite)
    for &suffix in &["어", "아", "여"] {
        if q.ends_with(suffix) && q.chars().count() > suffix.chars().count() {
            let stem = drop_last_n(&q, suffix.chars().count());
            if !stem.is_empty() {
                candidates.insert(format!("{}다", stem));
            }
        }
    }

    // Past polite: -었어요/-았어요/-였어요
    for &suffix in &["었어요", "았어요", "였어요"] {
        if q.ends_with(suffix) && q.chars().count() > suffix.chars().count() {
            let stem = drop_last_n(&q, suffix.chars().count());
            if !stem.is_empty() { candidates.insert(format!("{}다", stem)); }
        }
    }

    // Past formal: -었습니다/-았습니다/-였습니다
    for &suffix in &["었습니다", "았습니다", "였습니다"] {
        if q.ends_with(suffix) && q.chars().count() > suffix.chars().count() {
            let stem = drop_last_n(&q, suffix.chars().count());
            if !stem.is_empty() { candidates.insert(format!("{}다", stem)); }
        }
    }

    // Future/prospective: -겠다, -겠어요, -겠습니다
    if q.ends_with("겠습니다") && q.chars().count() > 5 {
        candidates.insert(format!("{}다", drop_last_n(&q, 5)));
    }
    if q.ends_with("겠어요") && q.chars().count() > 4 {
        candidates.insert(format!("{}다", drop_last_n(&q, 4)));
    }
    if q.ends_with("겠다") && q.chars().count() > 2 {
        candidates.insert(format!("{}다", drop_last_n(&q, 2)));
    }

    // If nothing matched but query ends with 다 (already dictionary form), just keep it
    if q.ends_with('다') && q.len() > 1 {
        candidates.insert(q.clone());
    }

    // Dedup and limit
    candidates.into_iter()
        .filter(|c| c.len() >= 2)
        .take(20)
        .collect()
}

// ── Substring fallback search ─────────────────────────────────

fn substring_search(entries: &[(String, String)], query: &str, max_results: usize) -> Vec<DictionaryEntry> {
    let q = query.trim().to_lowercase();
    if q.len() < 2 { return vec![]; }

    // Find insertion point for the query
    let ins = entries.binary_search_by(|(w, _)| w.to_lowercase().cmp(&q))
        .unwrap_or_else(|i| i);

    let mut results = Vec::new();
    let mut seen = BTreeSet::new();
    let len = entries.len();

    // Scan forward from insertion point
    let mut i = ins;
    while i < len && results.len() < max_results {
        let (w, d) = &entries[i];
        let w_lower = w.to_lowercase();
        if w_lower.contains(&q) {
            if seen.insert(w_lower.clone()) {
                results.push(DictionaryEntry {
                    headword: w.clone(), lemma: w.clone(), forms: vec![],
                    definition_html: d.clone(), matched_terms: vec![query.to_string()],
                });
            }
        }
        i += 1;
        // Stop after scanning a reasonable window (5000 entries)
        if i - ins > 5000 { break; }
    }

    // Scan backward from insertion point
    let mut i = if ins > 0 { ins - 1 } else { 0 };
    let mut scanned = 0;
    while scanned < 5000 && results.len() < max_results {
        let (w, d) = &entries[i];
        let w_lower = w.to_lowercase();
        if w_lower.contains(&q) {
            if seen.insert(w_lower.clone()) {
                results.push(DictionaryEntry {
                    headword: w.clone(), lemma: w.clone(), forms: vec![],
                    definition_html: d.clone(), matched_terms: vec![query.to_string()],
                });
            }
        }
        if i == 0 { break; }
        i -= 1;
        scanned += 1;
    }

    results
}

fn search_tsv_blocking(
    app: &AppHandle, query: &str,
    svc_fn: &dyn Fn(&AppHandle) -> Result<&'static Mutex<TsvDictService>, String>,
) -> Result<DictionarySearchResponse, String> {
    let cleaned = sanitize_query(query);
    let nq = normalize_lookup_key(&cleaned);
    if cleaned.is_empty() { return Ok(empty_resp(query, &nq)); }
    let svc = svc_fn(app)?;
    let svc = svc.lock().map_err(|_| "tsv mutex")?;
    Ok(DictionarySearchResponse { query: query.to_string(), normalized_query: nq, results: svc.search(&cleaned) })
}

// ═══════════════════════════════════════════ Shared utilities

fn dict_dir(app: &AppHandle, subdir: &str) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join(subdir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn empty_resp(query: &str, nq: &str) -> DictionarySearchResponse {
    DictionarySearchResponse { query: query.to_string(), normalized_query: nq.to_string(), results: vec![] }
}

fn find_exact_range(list: &[KeyWordItem], target: &str) -> Result<Option<(usize, usize)>, String> {
    if list.is_empty() { return Ok(None); }
    let start = lower_bound(list, target)?;
    if start >= list.len() || normalize_lookup_key(&list[start].key_text) != target { return Ok(None); }
    let end = upper_bound(list, target)?;
    Ok(Some((start, end)))
}
fn lower_bound(list: &[KeyWordItem], target: &str) -> Result<usize, String> {
    let mut lo = 0; let mut hi = list.len();
    while lo < hi { let mid = (lo+hi)/2; if normalize_lookup_key(&list[mid].key_text).as_str() < target { lo = mid+1; } else { hi = mid; } }
    Ok(lo)
}
fn upper_bound(list: &[KeyWordItem], target: &str) -> Result<usize, String> {
    let mut lo = 0; let mut hi = list.len();
    while lo < hi { let mid = (lo+hi)/2; if normalize_lookup_key(&list[mid].key_text).as_str() <= target { lo = mid+1; } else { hi = mid; } }
    Ok(lo)
}
fn sanitize_query(s: &str) -> String { s.replace('\u{0301}', "").trim().to_string() }
fn normalize_lookup_key(s: &str) -> String {
    sanitize_query(s).to_lowercase().chars().filter(|ch| !matches!(ch, '('|')'|'.'|','|'-'|'&'|' '|'\''|'/'|'\\'|'@'|'_'|'$'|'!')).collect()
}
fn cleanup_html(html: &str) -> String {
    let s = html.replace("@@@LINK=", "").replace("@@@", "");
    let mut out: Vec<String> = Vec::new(); let mut prev: Option<String> = None;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() { if out.last().map(|l: &String| !l.is_empty()).unwrap_or(false) { out.push(String::new()); } continue; }
        if prev.as_deref() == Some(t) { continue; }
        prev = Some(t.to_string()); out.push(t.to_string());
    }
    out.join("\n")
}
fn merge_entries(entries: Vec<DictionaryEntry>) -> Vec<DictionaryEntry> {
    let mut m: BTreeMap<String, DictionaryEntry> = BTreeMap::new();
    for e in entries {
        let k = { let lk = normalize_lookup_key(&e.lemma); if lk.is_empty() { normalize_lookup_key(&e.headword) } else { lk } };
        m.entry(k).and_modify(|ex| {
            if ex.headword.is_empty() { ex.headword = e.headword.clone(); }
            if ex.lemma.is_empty() { ex.lemma = e.lemma.clone(); }
            merge_vec(&mut ex.forms, &e.forms);
            merge_vec(&mut ex.matched_terms, &e.matched_terms);
            if !e.definition_html.is_empty() {
                if !ex.definition_html.is_empty() { ex.definition_html.push_str("<hr class=\"my-3 border-zinc-200 dark:border-zinc-800\" />"); }
                ex.definition_html.push_str(&e.definition_html);
            }
        }).or_insert(e);
    }
    m.into_values().collect()
}
fn merge_vec(v: &mut Vec<String>, add: &[String]) { for x in add { if !v.iter().any(|y| y==x) { v.push(x.clone()); } } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_spanish_html() {
        let re_c = Regex::new(r"<c>([^<]*)</c>").unwrap();
        let re_ipa = Regex::new(r"<i>//([^/]*)//</i>").unwrap();
        let re_ipa2 = Regex::new(r"<i>/([^/]+)/</i>").unwrap();
        let re_fn = Regex::new(r"<i>\(([^)]*)\)</i>").unwrap();
        let re_rem_i = Regex::new(r"</?i>").unwrap();

        // Test basic noun entry
        let input = r#"<d><h1>casa <c>noun</c></h1><i>//ˈkasa//</i><m><u><li>house</li></u></m><f><s><b>casas</b> <i>(plural)</i></s></f></d>"#;
        let result = transform_spanish_html(input, &re_c, &re_ipa, &re_ipa2, &re_fn, &re_rem_i);
        assert!(result.contains(r#"<div class="mdx-entry">"#));
        assert!(result.contains(r#"<span class="pos">noun</span>"#));
        assert!(result.contains(r#"<span class="ipa">[ˈkasa]</span>"#));
        assert!(result.contains(r#"<h2>Definitions</h2>"#));
        assert!(result.contains("<li>house</li>"));
        assert!(result.contains(r#"<h2>Forms</h2>"#));
        assert!(result.contains(r#"<span class="form-item"><b>casas</b> <em>plural</em></span>"#));
        assert!(!result.contains("<d>"));
        assert!(!result.contains("<c>"));
        assert!(!result.contains("<m>"));
        assert!(!result.contains("<f>"));
        assert!(!result.contains("<s>"));

        // Test homonym with hr separator
        let input2 = r#"<d><h1>casa <c>noun</c></h1><m><u><li>house</li></u></m></d><hr/><d><h1>casa <c>verb</c></h1><m><u><li>inflection of casar</li></u></m></d>"#;
        let result2 = transform_spanish_html(input2, &re_c, &re_ipa, &re_ipa2, &re_fn, &re_rem_i);
        assert!(result2.contains(r#"<hr class="mdx-sep" />"#));
        assert_eq!(result2.matches("mdx-entry").count(), 2);
    }

    #[test]
    fn test_spanish_candidates() {
        // "hablo" (1st person present of "hablar")
        let cands = generate_spanish_candidates("hablo");
        assert!(cands.iter().any(|c| c == "hablar"));

        // "gatos" (plural of "gato")
        let cands = generate_spanish_candidates("gatos");
        assert!(cands.iter().any(|c| c == "gato"));

        // "comiendo" (gerund of "comer")
        let cands = generate_spanish_candidates("comiendo");
        assert!(cands.iter().any(|c| c == "comer"));

        // "buenas" (feminine plural of "bueno")
        let cands = generate_spanish_candidates("buenas");
        assert!(cands.iter().any(|c| c == "bueno"));
    }

    #[test]
    fn test_korean_candidates() {
        // "학교에" (school + location marker)
        let cands = generate_korean_candidates("학교에");
        assert!(cands.iter().any(|c| c == "학교"));

        // "사과를" (apple + object marker)
        let cands = generate_korean_candidates("사과를");
        assert!(cands.iter().any(|c| c == "사과"));

        // "먹었어요" (ate + polite informal past)
        let cands = generate_korean_candidates("먹었어요");
        assert!(cands.iter().any(|c| c == "먹다"));

        // "가요" (go + polite informal) -- this is tricky, let's check
        let cands = generate_korean_candidates("가요");
        // 가 + 여요 suffix... should generate "가다"
        assert!(cands.iter().any(|c| c == "가다"));
    }
}
