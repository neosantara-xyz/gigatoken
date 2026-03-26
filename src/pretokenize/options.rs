pub enum PretokenizerType {
    GPT2, // Also used by llama, also known as r50k
    GPT4,
    Qwen2,      // Slightly adapted from GPT4, also used by Qwen3
    DeepSeekV3, // o200k, also used by GPT-4o
}

// impl TryFrom<&str> for PretokenizerType {
//     type Error = String;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         match value.to_lowercase().as_str() {
//             "gpt2" => Ok(PretokenizerType::GPT2),
//             "gpt4" => Ok(PretokenizerType::GPT4),
//             "qwen2" => Ok(PretokenizerType::Qwen2),
//             "deepseekv3" => Ok(PretokenizerType::DeepSeekV3),
//             _ => Err(format!("Unknown pretokenizer type: {}", value)),
//         }
//     }
// }

impl PretokenizerType {}
