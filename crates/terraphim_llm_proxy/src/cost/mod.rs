//! Cost management and optimization module
//!
//! This module provides cost-aware routing capabilities including:
//! - Pricing database for different providers and models
//! - Cost calculation for requests
//! - Budget management and limits
//! - Cost-optimized routing decisions

pub mod calculator;
pub mod config;
pub mod database;
pub mod manager;

pub use calculator::{CostCalculator, CostEstimate};
pub use config::{CostConfig, PricingInfo};
pub use database::{ModelPricing, PricingDatabase};
pub use manager::{BudgetAlert, BudgetInfo, BudgetManager};
