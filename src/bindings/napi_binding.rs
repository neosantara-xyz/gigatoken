use crate::batch::{encode_into, WorkerPool};
use crate::bpe::Tokenizer;
use crate::load_tokenizer;
use crate::pretokenize;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::PathBuf;

#[napi]
pub struct GigaTokenizer {
  tokenizer: Tokenizer,
  workers: WorkerPool,
}

#[napi]
impl GigaTokenizer {
  #[napi(constructor)]
  pub fn new(name_or_path: Option<String>) -> napi::Result<Self> {
    let target = name_or_path.unwrap_or_else(|| "openai-community/gpt2".to_string());
    Self::from_name(target)
  }

  #[napi(factory)]
  pub fn from_name(name_or_path: String) -> napi::Result<Self> {
    let path_buf = PathBuf::from(&name_or_path);
    if path_buf.exists() {
      if name_or_path.ends_with(".tiktoken") {
        return Self::from_tiktoken(name_or_path);
      }
      return Self::from_hf(name_or_path);
    }

    let hub_path = load_tokenizer::hub::hub_file_in(
      load_tokenizer::hub::RepoType::Model,
      &name_or_path,
      "tokenizer.json",
      "main",
    ).map_err(|e| napi::Error::from_reason(format!("Failed to load tokenizer from HuggingFace Hub for '{}': {}", name_or_path, e)))?;

    let tokenizer = load_tokenizer::hf::load_hf_bpe(&hub_path)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(Self {
      tokenizer,
      workers: WorkerPool::new(),
    })
  }

  #[napi(factory)]
  pub fn from_tiktoken(path: String) -> napi::Result<Self> {
    let path_buf = PathBuf::from(path);
    let tokenizer = load_tokenizer::tiktoken::load_tiktoken(&path_buf)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self {
      tokenizer,
      workers: WorkerPool::new(),
    })
  }

  #[napi(factory)]
  pub fn from_tiktoken_model(
    model_path: String,
    config_path: String,
    pretokenizer: String,
  ) -> napi::Result<Self> {
    let m_path = PathBuf::from(model_path);
    let c_path = PathBuf::from(config_path);
    let scheme = pretokenize::PretokenizerType::from_name(&pretokenizer).ok_or_else(|| {
      napi::Error::from_reason(format!("unknown pretokenizer scheme {:?}", pretokenizer))
    })?;
    let tokenizer =
      load_tokenizer::tiktoken::load_tiktoken_model(&m_path, &c_path, scheme)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self {
      tokenizer,
      workers: WorkerPool::new(),
    })
  }

  #[napi(factory)]
  pub fn from_hf(path: String) -> napi::Result<Self> {
    let path_buf = PathBuf::from(path);
    let tokenizer = load_tokenizer::hf::load_hf_bpe(&path_buf)
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self {
      tokenizer,
      workers: WorkerPool::new(),
    })
  }

  #[napi]
  pub fn encode(&mut self, text: String) -> Vec<u32> {
    let mut ids = Vec::new();
    let mut lens = Vec::new();
    encode_into(&mut self.tokenizer, text.as_bytes(), &mut ids, &mut lens);
    ids
  }

  #[napi]
  pub fn count_tokens(&mut self, text: String) -> u32 {
    let mut ids = Vec::new();
    let mut lens = Vec::new();
    encode_into(&mut self.tokenizer, text.as_bytes(), &mut ids, &mut lens);
    ids.len() as u32
  }

  #[napi]
  pub fn count_tokens_batch(&mut self, inputs: Vec<String>) -> Vec<u32> {
    inputs
      .into_iter()
      .map(|input| {
        let mut ids = Vec::new();
        let mut lens = Vec::new();
        encode_into(&mut self.tokenizer, input.as_bytes(), &mut ids, &mut lens);
        ids.len() as u32
      })
      .collect()
  }
}
