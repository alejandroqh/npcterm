mod terminal;
mod input;
mod screen;
mod status;
mod manager;
mod mcp;

fn main() {
    mcp::server::run_stdio_server();
}
