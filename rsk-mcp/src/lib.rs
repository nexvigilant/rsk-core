//! # RSK MCP Server
//!
//! MCP server exposing the RSK computation kernel to AI agents.
//!
//! ## Tools (16)
//!
//! | Category | Count | Tools |
//! |----------|-------|-------|
//! | System | 1 | `rsk_health` |
//! | Microgram | 8 | `mcg_run`, `mcg_test`, `mcg_test_all`, `mcg_chain`, `mcg_chain_test`, `mcg_list`, `mcg_info`, `mcg_coverage` |
//! | Statistics | 4 | `stats_chi_square`, `stats_t_test`, `stats_proportion_test`, `stats_correlation` |
//! | Decision | 1 | `decision_tree_run` |
//! | Graph | 2 | `graph_topsort`, `graph_parallel_levels` |

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

pub mod params;
pub mod server;

pub use server::RskMcpServer;
