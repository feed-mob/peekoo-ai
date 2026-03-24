use anyhow::Result;
use peekoo_agent_acp::run_agent;

#[tokio::main]
async fn main() -> Result<()> {
    run_agent().await?;
    Ok(())
}
