# Aklypse Project Directory Tree

```markdown
aklypse/
├── .cargo/
│   └── config.toml
├── .vscode/
│   └── settings.json
├── build_scripts/
├── cicd/
│   └── cicd.sh
├── configs/
│   ├── default.toml
│   ├── development.toml
│   ├── production.toml
│   └── origami_devnet.toml
├── data/
│   ├── dkg_store/
│   ├── logs/
│   └── rl_models/
├── docs/
│   ├── architecture.md
│   ├── api_reference.md
│   └── deployment_guide.md
├── examples/
├── frontend/
│   ├── public/
│   ├── src/
│   │   ├── assets/
│   │   ├── components/
│   │   │   ├── common/
│   │   │   ├── dashboard/
│   │   │   ├── knowledge_graph/
│   │   │   ├── market_intel/
│   │   │   └── trading/
│   │   ├── contexts/
│   │   ├── hooks/
│   │   ├── layouts/
│   │   ├── pages/
│   │   ├── services/
│   │   ├── styles/
│   │   ├── types/
│   │   ├── utils/
│   │   ├── App.tsx
│   │   └── main.tsx
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
├── scripts/
├── src/
│   ├── arbitrage/
│   │   ├── aggregator.rs
│   │   ├── engine.rs
│   │   └── mod.rs
│   ├── cache/
│   │   ├── graph_cache.rs
│   │   ├── market_cache.rs
│   │   └── mod.rs
│   ├── common/                   # Common utilities, types, and errors across Aklypse
│   │   ├── error/                # Comprehensive error handling framework
│   │   │   ├── circuitbreaker.rs # Circuit breaker implementation
│   │   │   ├── decrust.rs        # Autocorrection suggestion logic
│   │   │   ├── reporter.rs       # Error reporting utilities
│   │   │   ├── types.rs          # Core error-related structs (ErrorContext, Severity, etc.)
│   │   │   └── mod.rs            # Main AklypseError enum (Snafu-based) & extensions & Exports for the error module
│   │   ├── data_types.rs         # General common data types (Price, OrderID, etc.)
│   │   ├── utils.rs              # General utility functions
│   │   └── mod.rs                # Main common module, re-exporting from submodules
│   ├── config/
│   │   ├── settings.rs
│   │   └── mod.rs
│   ├── core/
│   │   ├── engine.rs
│   │   ├── events.rs
│   │   └── mod.rs
│   ├── crypto/
│   │   ├── dkg_frost.rs
│   │   ├── key_management.rs
│   │   └── mod.rs
│   ├── data_pipeline/
│   │   ├── collectors/
│   │   │   └── solana_rpc_collector.rs
│   │   ├── processors/
│   │   └── mod.rs
│   ├── dex/
│   │   ├── dex_interface.rs
│   │   ├── jupiter.rs
│   │   ├── openbook.rs
│   │   ├── openbook_v2.rs
│   │   ├── phoenix.rs
│   │   ├── raydium.rs
│   │   ├── serum.rs
│   │   └── mod.rs
│   ├── dkg/
│   │   ├── common_error.rs       # DKG-specific error (can be refactored to use common::error)
│   │   ├── config.rs
│   │   ├── common_types.rs       # DKG-specific common types (can be refactored)
│   │   ├── dkg_types.rs
│   │   ├── ses_types.rs
│   │   ├── knowledge_graph_trait.rs
│   │   ├── diamond_dkg.rs
│   │   ├── ontology.rs
│   │   ├── adapters.rs
│   │   └── mod.rs
│   ├── hardware_accel/
│   │   ├── cuda_kernels.rs
│   │   ├── simd_ops.rs
│   │   └── mod.rs
│   ├── indicators/
│   │   ├── momentum_indicators.rs
│   │   ├── support_resistance.rs
│   │   ├── trend_indicators.rs
│   │   ├── volatility_indicators.rs
│   │   ├── volume_indicators.rs
│   │   └── mod.rs
│   ├── integration_middleware/
│   │   ├── message_broker.rs
│   │   └── mod.rs
│   ├── ml/
│   │   ├── environments/
│   │   │   └── solana_trading_env.rs
│   │   ├── agents/
│   │   │   └── ppo_agent.rs
│   │   ├── models/
│   │   ├── reward_shaping.rs
│   │   └── mod.rs
│   ├── origami_mode/
│   │   ├── engine.rs
│   │   ├── devnet_connector.rs
│   │   ├── rl_interface.rs
│   │   └── mod.rs
│   ├── portfolio_manager/
│   │   ├── portfolio.rs
│   │   ├── risk_metrics.rs
│   │   └── mod.rs
│   ├── preforeception/
│   │   ├── engine.rs
│   │   ├── adapters.rs
│   │   └── mod.rs
│   ├── research/
│   │   ├── arp_engine.rs
│   │   ├── nlp_processor.rs
│   │   ├── search_providers/
│   │   │   ├── brave_provider.rs
│   │   │   └── duckduckgo_provider.rs
│   │   ├── text_extractor.rs
│   │   ├── sentiment_analyzer.rs
│   │   └── mod.rs
│   ├── security/
│   │   ├── audit_logger.rs
│   │   ├── transaction_shield.rs
│   │   └── mod.rs
│   ├── signal_processor/
│   │   ├── feature_engineering.rs
│   │   └── mod.rs
│   ├── strategies/
│   │   ├── strategy_set1.rs
│   │   ├── strategy_set2.rs
│   │   ├── strategy_set3.rs
│   │   ├── strategy_set4.rs
│   │   ├── strategy_set5_ml.rs
│   │   ├── sentiment_enhanced_strategy.rs
│   │   ├── strategy_dsl.rs
│   │   ├── strategy_manager.rs
│   │   └── mod.rs
│   ├── tauri_bindings/
│   │   ├── commands.rs
│   │   ├── state.rs
│   │   └── mod.rs
│   ├── trading_engine/
│   │   ├── engine.rs
│   │   ├── order_manager.rs
│   │   └── mod.rs
│   ├── lib.rs
│   └── main.rs
├── target/
├── tests/
│   ├── common/
│   └── integration_tests.rs
├── .env.example
├── .gitignore
├── Cargo.toml
├── README.md
└── rust-toolchain.toml
```
