use llm_client::LlmError;
use llm_client::config::{LlmClientCfg, LlmRequestCfg};
use llm_client::llm::*;
use llm_client::openai::OpenAiRequest;

use std::io::Write;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), LlmError> {
    let auth = std::env::args().nth(1);
    println!("Welcome to LLM client REPL. Your token is: {auth:?}");

    println!("At main loop");
    run_main_loop::<OpenAiRequest>(auth).await?;
    println!("Goodbye, world!");
    Ok(())
}

async fn run_main_loop<T: CallLlmService>(auth: Option<String>) -> Result<(), LlmError> {
    let base_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let req_cfg_path = base_path.clone().join("llm_request_cfg.toml");
    let client_cfg_path = base_path.clone().join("llm_client_config.toml");

    println!("Loading manifests");
    let client_config = LlmClientCfg::from_file(&client_cfg_path)?;
    let cfg = LlmRequestCfg::from_file(&req_cfg_path)?;

    // Auth is now optional!
    println!("Creating client");
    let c = match auth {
        Some(auth) => T::Client::new()?.set_auth(auth),
        None => T::Client::new()?,
    }
    .set_base_uri(client_config.get_base_url())?;

    println!("Creating request");
    let mut req = <T as LlmRequest>::new();
    req = cfg.configure(req);
    println!("{req:#?}");
    loop {
        print!(">>> ");
        _ = std::io::stdout().flush();
        // Get the question.
        let mut q = String::new();
        _ = std::io::stdin().read_line(&mut q);
        q = q.trim().to_string();
        // A little bit of flow control.
        if q.to_lowercase() == "quit" {
            break;
        } else if q.is_empty() {
            continue;
        }
        // Add message
        let m = <T as LlmRequest>::Message::new_user(q.trim());
        req.add_message(m);
        // Send the message
        match req.post(&c, client_config.get_chat_path()).await {
            Ok(r) => {
                let Some(message) = r.take_messages().pop() else {
                    continue;
                };
                println!("{}", message.content());
                req.add_message(message.to_assist());
            }
            Err(e) => {
                println!("Could not work with LLM: {e}");
            }
        };
    }
    Ok(())
}
