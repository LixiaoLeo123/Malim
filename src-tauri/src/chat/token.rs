use tiktoken_rs::cl100k_base;

pub fn count_tokens(text: &str) -> usize {
    cl100k_base()
        .map(|bpe| bpe.encode_with_special_tokens(text).len())
        .unwrap_or(0)
}
