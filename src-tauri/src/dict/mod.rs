use rsmorphy::opencorpora::Dictionary;
use rsmorphy::MorphAnalyzer;
use rsmorphy::Source;
use rust_mdict::{KeyWordItem, Mdd, Mdx};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

struct DictionaryService {
	mdx: Mdx,
	_mdd: Option<Mdd>,
	morph: MorphAnalyzer,
}

static DICTIONARY_SERVICE: OnceLock<Result<Mutex<DictionaryService>, String>> = OnceLock::new();

const EMBEDDED_MDX: &[u8] = include_bytes!("assets/OpenRussian.mdx");
const EMBEDDED_MDD: &[u8] = include_bytes!("assets/OpenRussian.mdd");

#[tauri::command]
pub async fn search_russian_dictionary(
	app: AppHandle,
	query: String,
) -> Result<DictionarySearchResponse, String> {
	tokio::task::spawn_blocking(move || search_russian_dictionary_blocking(&app, query))
		.await
		.map_err(|error| error.to_string())?
}

fn search_russian_dictionary_blocking(
	app: &AppHandle,
	query: String,
) -> Result<DictionarySearchResponse, String> {
	let cleaned_query = sanitize_query(&query);
	let normalized_query = normalize_lookup_key(&cleaned_query);
	if normalized_query.is_empty() {
		return Ok(DictionarySearchResponse {
			query,
			normalized_query,
			results: Vec::new(),
		});
	}

	let service = dictionary_service(app)?;
	let mut service = service
		.lock()
		.map_err(|_| "dictionary service mutex poisoned".to_string())?;
	let lemmatized_query = lemmatize_lookup_query(&service.morph, &cleaned_query);
	let normalized_query = normalize_lookup_key(&lemmatized_query);
	if normalized_query.is_empty() {
		return Ok(DictionarySearchResponse {
			query,
			normalized_query,
			results: Vec::new(),
		});
	}

	let mut lookup_terms = Vec::new();
	lookup_terms.push(lemmatized_query.clone());
	if normalize_lookup_key(&cleaned_query) != normalized_query {
		lookup_terms.push(cleaned_query.clone());
	}
	for candidate in build_lookup_candidates(&service.morph, &lemmatized_query) {
		if normalize_lookup_key(&candidate) != normalized_query {
			lookup_terms.push(candidate);
		}
	}

	let mut raw_results = Vec::new();
	let mut seen = BTreeSet::new();
	let mut selected_candidate: Option<String> = None;

	for candidate in lookup_terms {
		let normalized_candidate = normalize_lookup_key(&candidate);
		if normalized_candidate.is_empty() {
			continue;
		}

		let Some((start, end)) = find_exact_range(service.mdx.keyword_list(), &normalized_candidate)? else {
			continue;
		};

		selected_candidate = Some(candidate.clone());

		for index in start..end {
			let key_item = service
				.mdx
				.keyword_list()
				.get(index)
				.cloned()
				.ok_or_else(|| format!("keyword index {index} is out of range"))?;
			let dedupe_key = format!("{}|{}", candidate, key_item.key_text);
			if !seen.insert(dedupe_key) {
				continue;
			}

			let entry = build_dictionary_entry(&mut service, &key_item, &candidate)?;
			raw_results.push(entry);
		}
		break;
	}

	if selected_candidate.is_none() {
		return Ok(DictionarySearchResponse {
			query,
			normalized_query,
			results: Vec::new(),
		});
	}

	let results = merge_dictionary_entries(raw_results);

	Ok(DictionarySearchResponse {
		query,
		normalized_query,
		results,
	})
}

fn lemmatize_lookup_query(morph: &MorphAnalyzer, query: &str) -> String {
	let cleaned = sanitize_query(query);
	if cleaned.is_empty() {
		return cleaned;
	}

	let parse_result = catch_unwind(AssertUnwindSafe(|| morph.parse(&cleaned)));

	match parse_result {
		Ok(parsed_vec) => parsed_vec
			.first()
			.map(|parsed| sanitize_query(&parsed.lex.get_lemma(morph).get_word()))
			.filter(|lemma| !lemma.is_empty())
			.unwrap_or(cleaned),
		Err(_) => cleaned,
	}
}

fn dictionary_service(app: &AppHandle) -> Result<&'static Mutex<DictionaryService>, String> {
	let init_result = DICTIONARY_SERVICE.get_or_init(|| DictionaryService::new(app).map(Mutex::new));
	init_result.as_ref().map_err(|error| error.clone())
}

impl DictionaryService {
	fn new(app: &AppHandle) -> Result<Self, String> {
		let (mdx_path, mdd_path) = ensure_dictionary_files(app)?;
		let morph = load_russian_morphology(app)?;
		let mdx = Mdx::new(&mdx_path).map_err(|error| error.to_string())?;
		let mdd = Mdd::new(&mdd_path)
			.map(Some)
			.unwrap_or_else(|error| {
				eprintln!("Failed to open OpenRussian.mdd: {error}");
				None
			});

		Ok(Self {
			mdx,
			_mdd: mdd,
			morph,
		})
	}
}

fn load_russian_morphology(app: &AppHandle) -> Result<MorphAnalyzer, String> {
	let dict_path = crate::memory::ensure_dict_files(app);
	let dict = Dictionary::from_file(dict_path);
	Ok(MorphAnalyzer::new(dict))
}

fn ensure_dictionary_files(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
	let app_data = app
		.path()
		.app_data_dir()
		.map_err(|error| error.to_string())?;
	let dict_dir = app_data.join("openrussian_dictionary");
	std::fs::create_dir_all(&dict_dir).map_err(|error| error.to_string())?;

	let mdx_path = dict_dir.join("OpenRussian.mdx");
	let mdd_path = dict_dir.join("OpenRussian.mdd");

	if !mdx_path.exists() {
		std::fs::write(&mdx_path, EMBEDDED_MDX).map_err(|error| error.to_string())?;
	}
	if !mdd_path.exists() {
		std::fs::write(&mdd_path, EMBEDDED_MDD).map_err(|error| error.to_string())?;
	}

	Ok((mdx_path, mdd_path))
}

fn build_lookup_candidates(morph: &MorphAnalyzer, query: &str) -> Vec<String> {
	let mut candidates = BTreeSet::new();
	let cleaned = sanitize_query(query);

	if !cleaned.is_empty() {
		candidates.insert(cleaned.clone());
	}

	if cleaned.contains('ё') {
		candidates.insert(cleaned.replace('ё', "е"));
	}
	if cleaned.contains('е') {
		candidates.insert(cleaned.replace('е', "ё"));
	}

	let parse_result = catch_unwind(AssertUnwindSafe(|| morph.parse(&cleaned)));

	match parse_result {
		Ok(parsed_vec) => {
			for parsed in parsed_vec.into_iter().take(8) {
				let lemma = sanitize_query(&parsed.lex.get_lemma(morph).get_word());
				if !lemma.is_empty() {
					candidates.insert(lemma);
				}

				let normal_form = sanitize_query(&parsed.lex.get_normal_form(morph));
				if !normal_form.is_empty() {
					candidates.insert(normal_form);
				}

				for lex in parsed.lex.get_lexeme(morph).into_iter().take(64) {
					let form = sanitize_query(&lex.get_word());
					if !form.is_empty() {
						candidates.insert(form);
					}
				}
			}
		}
		Err(_) => {
			eprintln!("rsmorphy parse panicked for lookup candidate: {cleaned}");
		}
	}

	candidates.into_iter().collect()
}

fn build_dictionary_entry(
	service: &mut DictionaryService,
	item: &KeyWordItem,
	matched_term: &str,
) -> Result<DictionaryEntry, String> {
	let lookup = service
		.mdx
		.fetch(item)
		.ok_or_else(|| format!("missing definition for {}", item.key_text))?;

	let headword = lookup.key_text.to_string();
	let definition_html = cleanup_dictionary_markup(lookup.definition);
	let (lemma, forms) = expand_lexeme(&service.morph, &headword);

	Ok(DictionaryEntry {
		headword,
		lemma,
		forms,
		definition_html,
		matched_terms: vec![matched_term.to_string()],
	})
}

fn expand_lexeme(morph: &MorphAnalyzer, headword: &str) -> (String, Vec<String>) {
	let mut lemma = sanitize_query(headword);
	let mut forms = BTreeSet::new();

	let parse_result = catch_unwind(AssertUnwindSafe(|| morph.parse(headword)));

	match parse_result {
		Ok(parsed_vec) => {
			for parsed in parsed_vec.into_iter().take(8) {
				lemma = sanitize_query(&parsed.lex.get_lemma(morph).get_word());
				for lex in parsed.lex.get_lexeme(morph) {
					let form = sanitize_query(&lex.get_word());
					if !form.is_empty() {
						forms.insert(form);
					}
				}
			}
		}
		Err(_) => {
			eprintln!("rsmorphy parse panicked for lexeme expansion: {headword}");
		}
	}

	if forms.is_empty() {
		forms.insert(lemma.clone());
	}

	if lemma.is_empty() {
		lemma = sanitize_query(headword);
	}

	(lemma, forms.into_iter().collect())
}

fn merge_dictionary_entries(entries: Vec<DictionaryEntry>) -> Vec<DictionaryEntry> {
	let mut grouped: BTreeMap<String, DictionaryEntry> = BTreeMap::new();

	for entry in entries {
		let group_key = {
			let lemma_key = normalize_lookup_key(&entry.lemma);
			if lemma_key.is_empty() {
				normalize_lookup_key(&entry.headword)
			} else {
				lemma_key
			}
		};

		grouped
			.entry(group_key)
			.and_modify(|existing| {
				if existing.headword.is_empty() {
					existing.headword = entry.headword.clone();
				}
				if existing.lemma.is_empty() {
					existing.lemma = entry.lemma.clone();
				}
				merge_string_vec(&mut existing.forms, &entry.forms);
				merge_string_vec(&mut existing.matched_terms, &entry.matched_terms);
				if !entry.definition_html.is_empty() {
					if !existing.definition_html.is_empty() {
						existing.definition_html.push_str("<hr class=\"my-3 border-zinc-200 dark:border-zinc-800\" />");
					}
					existing.definition_html.push_str(&entry.definition_html);
				}
			})
			.or_insert(entry);
	}

	grouped.into_values().collect()
}

fn merge_string_vec(existing: &mut Vec<String>, incoming: &[String]) {
	for item in incoming {
		if !existing.iter().any(|value| value == item) {
			existing.push(item.clone());
		}
	}
}

fn find_exact_range(
	keyword_list: &[KeyWordItem],
	normalized_query: &str,
) -> Result<Option<(usize, usize)>, String> {
	if keyword_list.is_empty() {
		return Ok(None);
	}

	let start = lower_bound(keyword_list, normalized_query)?;
	if start >= keyword_list.len() {
		return Ok(None);
	}

	if normalize_lookup_key(&keyword_list[start].key_text) != normalized_query {
		return Ok(None);
	}

	let end = upper_bound(keyword_list, normalized_query)?;
	Ok(Some((start, end)))
}

fn lower_bound(
	keyword_list: &[KeyWordItem],
	target: &str,
) -> Result<usize, String> {
	let mut left = 0usize;
	let mut right = keyword_list.len();

	while left < right {
		let mid = (left + right) / 2;
		let normalized = normalize_lookup_key(&keyword_list[mid].key_text);

		if normalized.as_str() < target {
			left = mid + 1;
		} else {
			right = mid;
		}
	}

	Ok(left)
}

fn upper_bound(
	keyword_list: &[KeyWordItem],
	target: &str,
) -> Result<usize, String> {
	let mut left = 0usize;
	let mut right = keyword_list.len();

	while left < right {
		let mid = (left + right) / 2;
		let normalized = normalize_lookup_key(&keyword_list[mid].key_text);

		if normalized.as_str() <= target {
			left = mid + 1;
		} else {
			right = mid;
		}
	}

	Ok(left)
}

fn sanitize_query(input: &str) -> String {
	input.replace('\u{0301}', "").trim().to_string()
}

fn normalize_lookup_key(input: &str) -> String {
	sanitize_query(input)
		.to_lowercase()
		.chars()
		.filter(|ch| !matches!(ch, '(' | ')' | '.' | ',' | '-' | '&' | ' ' | '\'' | '/' | '\\' | '@' | '_' | '$' | '!'))
		.collect()
}

fn cleanup_dictionary_markup(html: String) -> String {
	let without_links = html
		.replace("@@@LINK=", "")
		.replace("@@@", "");

	let mut cleaned_lines = Vec::new();
	let mut previous_line: Option<String> = None;

	for line in without_links.lines() {
		let trimmed = line.trim();
		if trimmed.is_empty() {
			if cleaned_lines.last().map(|line: &String| !line.is_empty()).unwrap_or(false) {
				cleaned_lines.push(String::new());
			}
			continue;
		}
		if previous_line.as_deref() == Some(trimmed) {
			continue;
		}
		previous_line = Some(trimmed.to_string());
		cleaned_lines.push(trimmed.to_string());
	}

	cleaned_lines.join("\n")
}
