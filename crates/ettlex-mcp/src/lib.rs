//! EttleX MCP server library.
//!
//! Provides an in-process MCP (Model Context Protocol) thin-slice server
//! that acts as a transport-only adapter over the `ettlex-engine` command
//! surface. No business logic lives here.

pub mod auth;
pub mod canonical;
pub mod context;
pub mod error;
pub mod server;
pub mod tools;
