use borsh::{BorshDeserialize, BorshSerialize};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Script {
    pub router: RouterBlock,
    pub slabs: Vec<SlabBlock>,
    pub oracles: Vec<OracleBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct RouterBlock {
    pub name: String,
    pub collateral_assets: Vec<CollateralSpec>,
    pub portfolio_margin: Option<PortfolioMarginSpec>,
    pub cap_ttl_ms: Option<u64>,
    pub reservation_batch_ms: Option<u64>,
    pub capabilities: Vec<CapabilitySpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CollateralSpec {
    pub asset: String,
    pub vault_cap: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct PortfolioMarginSpec {
    pub model: String,
    pub correl_matrix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CapabilitySpec {
    pub name: String,
    pub asset: String,
    pub limit: u128,
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SlabBlock {
    pub name: String,
    pub maker_class: MakerClassSpec,
    pub matching: Option<MatchingSpec>,
    pub fee: FeeSpec,
    pub risk: RiskSpec,
    pub anti_toxicity: Option<AntiToxicitySpec>,
    pub batch_window_ms: Option<u64>,
    pub oracle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MakerClassSpec {
    pub class: String,
    pub allowance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MatchingSpec {
    pub fifo: bool,
    pub pending_promotion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct FeeSpec {
    pub maker_bps: u16,
    pub taker_bps: u16,
    pub rebate_delay_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct RiskSpec {
    pub imr_bps: u16,
    pub mmr_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct AntiToxicitySpec {
    pub kill_band_bps: u16,
    pub jit_penalty: bool,
    pub arg_tax_bps: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct OracleBlock {
    pub name: String,
    pub heartbeat_ms: u64,
    pub kill_band_router_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct Manifest {
    pub router: RouterManifest,
    pub slabs: Vec<SlabManifest>,
    pub oracles: Vec<OracleManifest>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct RouterManifest {
    pub id: String,
    pub reservation_batch_ms: Option<u64>,
    pub capabilities: Vec<CapabilitySchema>,
    pub cpi_descriptors: Vec<CpiDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct CapabilitySchema {
    pub name: String,
    pub asset: String,
    pub limit: u128,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct SlabManifest {
    pub name: String,
    pub id: String,
    pub oracle: Option<String>,
    pub batch_window_ms: Option<u64>,
    pub cpi_descriptors: Vec<CpiDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct OracleManifest {
    pub name: String,
    pub heartbeat_ms: u64,
    pub router_dependency: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct CpiDescriptor {
    pub module: String,
    pub entrypoint: String,
    pub accounts: Vec<String>,
    pub args: Vec<ArgumentDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize)]
pub struct ArgumentDescriptor {
    pub name: String,
    pub type_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    Syntax {
        line: usize,
        message: String,
    },
    UnexpectedToken {
        line: usize,
        token: String,
    },
    DuplicateRouter {
        line: usize,
    },
    MissingRouter,
    MissingBlockTerminator,
    DuplicateSlab {
        name: String,
    },
    DuplicateOracle {
        name: String,
    },
    UnknownStatement {
        line: usize,
        block: String,
        statement: String,
    },
    MissingField {
        line: usize,
        block: String,
        field: String,
    },
    UnknownOracleReference {
        slab: String,
        oracle: String,
    },
    RouterReferenceMismatch {
        router: String,
        reference: String,
    },
    BatchToleranceExceeded {
        slab: String,
        router_ms: u64,
        slab_ms: u64,
    },
    CapabilityTtlExceeded {
        capability: String,
        ttl: u64,
        router_ttl: u64,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::Syntax { line, message } => {
                write!(f, "syntax error on line {}: {}", line, message)
            }
            ScriptError::UnexpectedToken { line, token } => {
                write!(f, "unexpected token '{}' on line {}", token, line)
            }
            ScriptError::DuplicateRouter { line } => {
                write!(f, "duplicate router block declared on line {}", line)
            }
            ScriptError::MissingRouter => write!(f, "script is missing a ROUTER block"),
            ScriptError::MissingBlockTerminator => write!(f, "unterminated block in script"),
            ScriptError::DuplicateSlab { name } => {
                write!(f, "duplicate SLAB block named '{}'", name)
            }
            ScriptError::DuplicateOracle { name } => {
                write!(f, "duplicate ORACLE block named '{}'", name)
            }
            ScriptError::UnknownStatement {
                line,
                block,
                statement,
            } => {
                write!(
                    f,
                    "unknown statement '{}' in {} block on line {}",
                    statement, block, line
                )
            }
            ScriptError::MissingField { line, block, field } => {
                write!(
                    f,
                    "missing field '{}' for {} block on line {}",
                    field, block, line
                )
            }
            ScriptError::UnknownOracleReference { slab, oracle } => {
                write!(f, "slab '{}' references unknown oracle '{}'", slab, oracle)
            }
            ScriptError::RouterReferenceMismatch { router, reference } => {
                write!(
                    f,
                    "oracle kill band references '{}' but router is named '{}'",
                    reference, router
                )
            }
            ScriptError::BatchToleranceExceeded {
                slab,
                router_ms,
                slab_ms,
            } => write!(
                f,
                "slab '{}' batch window {}ms exceeds router batch {}ms by more than 10ms",
                slab, slab_ms, router_ms
            ),
            ScriptError::CapabilityTtlExceeded {
                capability,
                ttl,
                router_ttl,
            } => write!(
                f,
                "capability '{}' ttl {}ms exceeds router CAP_TTL {}ms",
                capability, ttl, router_ttl
            ),
        }
    }
}

impl std::error::Error for ScriptError {}

pub fn parse(script: &str) -> Result<Script, ScriptError> {
    let parsed = parse_impl(script)?;
    validate(&parsed)?;
    Ok(parsed)
}

pub fn validate(script: &Script) -> Result<(), ScriptError> {
    let mut slab_names = HashSet::new();
    for slab in &script.slabs {
        if !slab_names.insert(slab.name.clone()) {
            return Err(ScriptError::DuplicateSlab {
                name: slab.name.clone(),
            });
        }
    }

    let mut oracle_names = HashSet::new();
    for oracle in &script.oracles {
        if !oracle_names.insert(oracle.name.clone()) {
            return Err(ScriptError::DuplicateOracle {
                name: oracle.name.clone(),
            });
        }
    }

    let router_name = &script.router.name;

    for slab in &script.slabs {
        if let Some(ref oracle_name) = slab.oracle {
            if !oracle_names.contains(oracle_name) {
                return Err(ScriptError::UnknownOracleReference {
                    slab: slab.name.clone(),
                    oracle: oracle_name.clone(),
                });
            }
        }
        if let (Some(router_batch), Some(slab_batch)) =
            (script.router.reservation_batch_ms, slab.batch_window_ms)
        {
            let diff = router_batch.abs_diff(slab_batch);
            if diff > 10 {
                return Err(ScriptError::BatchToleranceExceeded {
                    slab: slab.name.clone(),
                    router_ms: router_batch,
                    slab_ms: slab_batch,
                });
            }
        }
    }

    if let Some(router_ttl) = script.router.cap_ttl_ms {
        for cap in &script.router.capabilities {
            if let Some(ttl) = cap.ttl_ms {
                if ttl > router_ttl {
                    return Err(ScriptError::CapabilityTtlExceeded {
                        capability: cap.name.clone(),
                        ttl,
                        router_ttl,
                    });
                }
            }
        }
    }

    for oracle in &script.oracles {
        if let Some(ref reference) = oracle.kill_band_router_ref {
            if reference != router_name {
                return Err(ScriptError::RouterReferenceMismatch {
                    router: router_name.clone(),
                    reference: reference.clone(),
                });
            }
        }
    }

    Ok(())
}

pub fn emit_manifest(script: &Script) -> Manifest {
    let router_id = route_id(&script.router.name);

    let mut router_accounts = vec!["router_state".to_string()];
    router_accounts.extend(
        script
            .router
            .collateral_assets
            .iter()
            .map(|asset| format!("vault::{}", asset.asset)),
    );

    let mut router_capabilities = Vec::new();
    for cap in &script.router.capabilities {
        let ttl_ms = cap.ttl_ms.or(script.router.cap_ttl_ms).unwrap_or_default();
        router_capabilities.push(CapabilitySchema {
            name: cap.name.clone(),
            asset: cap.asset.clone(),
            limit: cap.limit,
            ttl_ms,
        });
    }

    let router_descriptors = vec![
        CpiDescriptor {
            module: "router".to_string(),
            entrypoint: "reserve".to_string(),
            accounts: router_accounts.clone(),
            args: vec![
                ArgumentDescriptor {
                    name: "user".to_string(),
                    type_hint: "Pubkey".to_string(),
                },
                ArgumentDescriptor {
                    name: "slab".to_string(),
                    type_hint: "Hash".to_string(),
                },
                ArgumentDescriptor {
                    name: "qty".to_string(),
                    type_hint: "u64".to_string(),
                },
            ],
        },
        CpiDescriptor {
            module: "router".to_string(),
            entrypoint: "commit".to_string(),
            accounts: router_accounts.clone(),
            args: vec![
                ArgumentDescriptor {
                    name: "reservation".to_string(),
                    type_hint: "Hash".to_string(),
                },
                ArgumentDescriptor {
                    name: "fill".to_string(),
                    type_hint: "Fill".to_string(),
                },
            ],
        },
        CpiDescriptor {
            module: "router".to_string(),
            entrypoint: "cancel".to_string(),
            accounts: router_accounts.clone(),
            args: vec![ArgumentDescriptor {
                name: "reservation".to_string(),
                type_hint: "Hash".to_string(),
            }],
        },
        CpiDescriptor {
            module: "router".to_string(),
            entrypoint: "liquidation_call".to_string(),
            accounts: router_accounts,
            args: vec![
                ArgumentDescriptor {
                    name: "user".to_string(),
                    type_hint: "Pubkey".to_string(),
                },
                ArgumentDescriptor {
                    name: "slab".to_string(),
                    type_hint: "Hash".to_string(),
                },
            ],
        },
    ];

    let slabs = script
        .slabs
        .iter()
        .map(|slab| {
            let slab_accounts = vec![
                format!("slab::{}", slab.name),
                format!("escrow::{}", slab.name),
            ];
            let mut args = vec![
                ArgumentDescriptor {
                    name: "user".to_string(),
                    type_hint: "Pubkey".to_string(),
                },
                ArgumentDescriptor {
                    name: "qty".to_string(),
                    type_hint: "u64".to_string(),
                },
            ];
            if slab.oracle.is_some() {
                args.push(ArgumentDescriptor {
                    name: "oracle_price".to_string(),
                    type_hint: "i64".to_string(),
                });
            }
            let descriptors = vec![
                CpiDescriptor {
                    module: slab.name.clone(),
                    entrypoint: "reserve".to_string(),
                    accounts: slab_accounts.clone(),
                    args: args.clone(),
                },
                CpiDescriptor {
                    module: slab.name.clone(),
                    entrypoint: "commit".to_string(),
                    accounts: slab_accounts.clone(),
                    args: vec![
                        ArgumentDescriptor {
                            name: "reservation".to_string(),
                            type_hint: "Hash".to_string(),
                        },
                        ArgumentDescriptor {
                            name: "fill".to_string(),
                            type_hint: "Fill".to_string(),
                        },
                    ],
                },
                CpiDescriptor {
                    module: slab.name.clone(),
                    entrypoint: "cancel".to_string(),
                    accounts: slab_accounts,
                    args: vec![ArgumentDescriptor {
                        name: "reservation".to_string(),
                        type_hint: "Hash".to_string(),
                    }],
                },
            ];

            SlabManifest {
                name: slab.name.clone(),
                id: route_id(&slab.name),
                oracle: slab.oracle.clone(),
                batch_window_ms: slab.batch_window_ms,
                cpi_descriptors: descriptors,
            }
        })
        .collect();

    let oracles = script
        .oracles
        .iter()
        .map(|oracle| OracleManifest {
            name: oracle.name.clone(),
            heartbeat_ms: oracle.heartbeat_ms,
            router_dependency: oracle.kill_band_router_ref.clone(),
        })
        .collect();

    Manifest {
        router: RouterManifest {
            id: router_id,
            reservation_batch_ms: script.router.reservation_batch_ms,
            capabilities: router_capabilities,
            cpi_descriptors: router_descriptors,
        },
        slabs,
        oracles,
    }
}

pub fn manifest_to_json(manifest: &Manifest) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(manifest)
}

pub fn route_id(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

pub fn hold_id(user: &str, slab: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user.as_bytes());
    hasher.update(b"::");
    hasher.update(slab.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..16])
}

fn parse_impl(script: &str) -> Result<Script, ScriptError> {
    enum BlockState {
        Router(RouterBuilder),
        Slab(SlabBuilder),
        Oracle(OracleBuilder),
    }

    let mut router: Option<RouterBlock> = None;
    let mut slabs = Vec::new();
    let mut oracles = Vec::new();
    let mut current: Option<BlockState> = None;

    for (idx, raw_line) in script.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let tokens = tokenize(trimmed);
        if tokens.is_empty() {
            continue;
        }
        if tokens.len() == 1 && tokens[0] == "}" {
            let state = current.take().ok_or(ScriptError::UnexpectedToken {
                line: line_no,
                token: "}".to_string(),
            })?;
            match state {
                BlockState::Router(builder) => {
                    if router.is_some() {
                        return Err(ScriptError::DuplicateRouter { line: line_no });
                    }
                    router = Some(builder.finish(line_no)?);
                }
                BlockState::Slab(builder) => slabs.push(builder.finish(line_no)?),
                BlockState::Oracle(builder) => oracles.push(builder.finish(line_no)?),
            }
            continue;
        }

        if current.is_none() {
            if tokens.last().map(|t| t.as_str()) != Some("{") {
                return Err(ScriptError::Syntax {
                    line: line_no,
                    message: "expected block opening".into(),
                });
            }
            let head_tokens = &tokens[..tokens.len() - 1];
            if head_tokens.is_empty() {
                return Err(ScriptError::Syntax {
                    line: line_no,
                    message: "missing block identifier".into(),
                });
            }
            match head_tokens[0].as_str() {
                "ROUTER" => {
                    let name = if head_tokens.len() > 1 {
                        head_tokens[1].clone()
                    } else {
                        "ROUTER".to_string()
                    };
                    current = Some(BlockState::Router(RouterBuilder::new(name)));
                }
                "SLAB" => {
                    if head_tokens.len() != 2 {
                        return Err(ScriptError::Syntax {
                            line: line_no,
                            message: "SLAB requires a quoted name".into(),
                        });
                    }
                    current = Some(BlockState::Slab(SlabBuilder::new(head_tokens[1].clone())));
                }
                "ORACLE" => {
                    if head_tokens.len() != 2 {
                        return Err(ScriptError::Syntax {
                            line: line_no,
                            message: "ORACLE requires a quoted name".into(),
                        });
                    }
                    current = Some(BlockState::Oracle(OracleBuilder::new(
                        head_tokens[1].clone(),
                    )));
                }
                other => {
                    return Err(ScriptError::Syntax {
                        line: line_no,
                        message: format!("unknown block '{}'", other),
                    });
                }
            }
            continue;
        }

        match current.as_mut().unwrap() {
            BlockState::Router(builder) => builder.apply(&tokens, line_no)?,
            BlockState::Slab(builder) => builder.apply(&tokens, line_no)?,
            BlockState::Oracle(builder) => builder.apply(&tokens, line_no)?,
        }
    }

    if current.is_some() {
        return Err(ScriptError::MissingBlockTerminator);
    }

    let router = router.ok_or(ScriptError::MissingRouter)?;

    Ok(Script {
        router,
        slabs,
        oracles,
    })
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        match ch {
            '"' => {
                if in_quotes {
                    tokens.push(current.clone());
                    current.clear();
                    in_quotes = false;
                } else {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    in_quotes = true;
                }
            }
            '{' | '}' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push(ch.to_string());
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_kv(
    tokens: &[String],
    _line: usize,
    _block: &str,
) -> Result<HashMap<String, String>, ScriptError> {
    let mut map = HashMap::new();
    let mut idx = 1;
    while idx < tokens.len() {
        let token = &tokens[idx];
        if let Some((key, value)) = token.split_once('=') {
            if !value.is_empty() {
                map.insert(key.to_string(), value.to_string());
            } else if idx + 1 < tokens.len() {
                map.insert(key.to_string(), tokens[idx + 1].clone());
                idx += 1;
            } else {
                map.insert(key.to_string(), String::new());
            }
        }
        idx += 1;
    }
    Ok(map)
}

fn parse_bool(value: &str, line: usize) -> Result<bool, ScriptError> {
    match value {
        "true" | "TRUE" => Ok(true),
        "false" | "FALSE" => Ok(false),
        _ => Err(ScriptError::Syntax {
            line,
            message: format!("expected boolean, got '{}'", value),
        }),
    }
}

fn parse_u16(value: &str, line: usize) -> Result<u16, ScriptError> {
    value.parse().map_err(|_| ScriptError::Syntax {
        line,
        message: format!("expected u16, got '{}'", value),
    })
}

fn parse_u64(value: &str, line: usize) -> Result<u64, ScriptError> {
    value.parse().map_err(|_| ScriptError::Syntax {
        line,
        message: format!("expected u64, got '{}'", value),
    })
}

fn parse_u128(value: &str, line: usize) -> Result<u128, ScriptError> {
    value.parse().map_err(|_| ScriptError::Syntax {
        line,
        message: format!("expected u128, got '{}'", value),
    })
}

#[derive(Default)]
struct RouterBuilder {
    name: String,
    collateral_assets: Vec<CollateralSpec>,
    portfolio_margin: Option<PortfolioMarginSpec>,
    cap_ttl_ms: Option<u64>,
    reservation_batch_ms: Option<u64>,
    capabilities: Vec<CapabilitySpec>,
}

impl RouterBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    fn apply(&mut self, tokens: &[String], line: usize) -> Result<(), ScriptError> {
        match tokens[0].as_str() {
            "COLLATERAL" => {
                let map = parse_kv(tokens, line, "ROUTER")?;
                let asset = map.get("asset").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "ROUTER".into(),
                    field: "asset".into(),
                })?;
                let vault_cap = map
                    .get("vault_cap")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "ROUTER".into(),
                        field: "vault_cap".into(),
                    })?;
                self.collateral_assets.push(CollateralSpec {
                    asset: asset.clone(),
                    vault_cap: parse_u64(vault_cap, line)?,
                });
            }
            "PORTFOLIO_MARGIN" => {
                let map = parse_kv(tokens, line, "ROUTER")?;
                let model = map
                    .get("model")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "ROUTER".into(),
                        field: "model".into(),
                    })?
                    .clone();
                let correl = map
                    .get("correl_matrix")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "ROUTER".into(),
                        field: "correl_matrix".into(),
                    })?
                    .clone();
                self.portfolio_margin = Some(PortfolioMarginSpec {
                    model,
                    correl_matrix: correl,
                });
            }
            "CAP_TTL" => {
                let map = parse_kv(tokens, line, "ROUTER")?;
                let ms = map.get("ms").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "ROUTER".into(),
                    field: "ms".into(),
                })?;
                self.cap_ttl_ms = Some(parse_u64(ms, line)?);
            }
            "RESERVATION_BATCH" => {
                let map = parse_kv(tokens, line, "ROUTER")?;
                let ms = map.get("ms").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "ROUTER".into(),
                    field: "ms".into(),
                })?;
                self.reservation_batch_ms = Some(parse_u64(ms, line)?);
            }
            "CAP" => {
                let map = parse_kv(tokens, line, "ROUTER")?;
                let name = map
                    .get("name")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "ROUTER".into(),
                        field: "name".into(),
                    })?
                    .clone();
                let asset = map
                    .get("asset")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "ROUTER".into(),
                        field: "asset".into(),
                    })?
                    .clone();
                let limit = map.get("limit").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "ROUTER".into(),
                    field: "limit".into(),
                })?;
                let ttl_ms = map.get("ttl_ms").map(|v| parse_u64(v, line)).transpose()?;
                self.capabilities.push(CapabilitySpec {
                    name,
                    asset,
                    limit: parse_u128(limit, line)?,
                    ttl_ms,
                });
            }
            other => {
                return Err(ScriptError::UnknownStatement {
                    line,
                    block: "ROUTER".into(),
                    statement: other.into(),
                });
            }
        }
        Ok(())
    }

    fn finish(self, line: usize) -> Result<RouterBlock, ScriptError> {
        if self.collateral_assets.is_empty() {
            return Err(ScriptError::MissingField {
                line,
                block: "ROUTER".into(),
                field: "COLLATERAL".into(),
            });
        }
        Ok(RouterBlock {
            name: self.name,
            collateral_assets: self.collateral_assets,
            portfolio_margin: self.portfolio_margin,
            cap_ttl_ms: self.cap_ttl_ms,
            reservation_batch_ms: self.reservation_batch_ms,
            capabilities: self.capabilities,
        })
    }
}

#[derive(Default)]
struct SlabBuilder {
    name: String,
    maker_class: Option<MakerClassSpec>,
    matching: Option<MatchingSpec>,
    fee: Option<FeeSpec>,
    risk: Option<RiskSpec>,
    anti_toxicity: Option<AntiToxicitySpec>,
    batch_window_ms: Option<u64>,
    oracle: Option<String>,
}

impl SlabBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    fn apply(&mut self, tokens: &[String], line: usize) -> Result<(), ScriptError> {
        match tokens[0].as_str() {
            "MAKER_CLASS" => {
                if tokens.len() < 2 {
                    return Err(ScriptError::Syntax {
                        line,
                        message: "MAKER_CLASS requires a class name".into(),
                    });
                }
                let class = tokens[1].clone();
                let map = parse_kv(tokens, line, "SLAB")?;
                let allowance = map
                    .get("allowance")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "allowance".into(),
                    })?;
                self.maker_class = Some(MakerClassSpec {
                    class,
                    allowance: parse_u64(allowance, line)?,
                });
            }
            "MATCHING" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let fifo = map.get("fifo").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "SLAB".into(),
                    field: "fifo".into(),
                })?;
                let pending =
                    map.get("pending_promotion")
                        .ok_or_else(|| ScriptError::MissingField {
                            line,
                            block: "SLAB".into(),
                            field: "pending_promotion".into(),
                        })?;
                self.matching = Some(MatchingSpec {
                    fifo: parse_bool(fifo, line)?,
                    pending_promotion: parse_bool(pending, line)?,
                });
            }
            "FEE" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let maker = map
                    .get("maker_bps")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "maker_bps".into(),
                    })?;
                let taker = map
                    .get("taker_bps")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "taker_bps".into(),
                    })?;
                let delay =
                    map.get("rebate_delay_ms")
                        .ok_or_else(|| ScriptError::MissingField {
                            line,
                            block: "SLAB".into(),
                            field: "rebate_delay_ms".into(),
                        })?;
                self.fee = Some(FeeSpec {
                    maker_bps: parse_u16(maker, line)?,
                    taker_bps: parse_u16(taker, line)?,
                    rebate_delay_ms: parse_u64(delay, line)?,
                });
            }
            "RISK" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let imr = map
                    .get("imr_bps")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "imr_bps".into(),
                    })?;
                let mmr = map
                    .get("mmr_bps")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "mmr_bps".into(),
                    })?;
                self.risk = Some(RiskSpec {
                    imr_bps: parse_u16(imr, line)?,
                    mmr_bps: parse_u16(mmr, line)?,
                });
            }
            "ANTI_TOXICITY" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let kill_band =
                    map.get("kill_band_bps")
                        .ok_or_else(|| ScriptError::MissingField {
                            line,
                            block: "SLAB".into(),
                            field: "kill_band_bps".into(),
                        })?;
                let jit = map
                    .get("jit_penalty")
                    .ok_or_else(|| ScriptError::MissingField {
                        line,
                        block: "SLAB".into(),
                        field: "jit_penalty".into(),
                    })?;
                let arg = map
                    .get("arg_tax_bps")
                    .map(|v| parse_u16(v, line))
                    .transpose()?;
                self.anti_toxicity = Some(AntiToxicitySpec {
                    kill_band_bps: parse_u16(kill_band, line)?,
                    jit_penalty: parse_bool(jit, line)?,
                    arg_tax_bps: arg,
                });
            }
            "BATCH_WINDOW" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let ms = map.get("ms").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "SLAB".into(),
                    field: "ms".into(),
                })?;
                self.batch_window_ms = Some(parse_u64(ms, line)?);
            }
            "ORACLE_LINK" => {
                let map = parse_kv(tokens, line, "SLAB")?;
                let id = map.get("id").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "SLAB".into(),
                    field: "id".into(),
                })?;
                self.oracle = Some(id.clone());
            }
            other => {
                return Err(ScriptError::UnknownStatement {
                    line,
                    block: "SLAB".into(),
                    statement: other.into(),
                });
            }
        }
        Ok(())
    }

    fn finish(self, line: usize) -> Result<SlabBlock, ScriptError> {
        Ok(SlabBlock {
            name: self.name,
            maker_class: self.maker_class.ok_or_else(|| ScriptError::MissingField {
                line,
                block: "SLAB".into(),
                field: "MAKER_CLASS".into(),
            })?,
            matching: self.matching,
            fee: self.fee.ok_or_else(|| ScriptError::MissingField {
                line,
                block: "SLAB".into(),
                field: "FEE".into(),
            })?,
            risk: self.risk.ok_or_else(|| ScriptError::MissingField {
                line,
                block: "SLAB".into(),
                field: "RISK".into(),
            })?,
            anti_toxicity: self.anti_toxicity,
            batch_window_ms: self.batch_window_ms,
            oracle: self.oracle,
        })
    }
}

#[derive(Default)]
struct OracleBuilder {
    name: String,
    heartbeat_ms: Option<u64>,
    kill_band_router_ref: Option<String>,
}

impl OracleBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }

    fn apply(&mut self, tokens: &[String], line: usize) -> Result<(), ScriptError> {
        match tokens[0].as_str() {
            "HEARTBEAT" => {
                let map = parse_kv(tokens, line, "ORACLE")?;
                let ms = map.get("ms").ok_or_else(|| ScriptError::MissingField {
                    line,
                    block: "ORACLE".into(),
                    field: "ms".into(),
                })?;
                self.heartbeat_ms = Some(parse_u64(ms, line)?);
            }
            "KILL_BAND_SYNC" => {
                let map = parse_kv(tokens, line, "ORACLE")?;
                let router_ref =
                    map.get("router_ref")
                        .ok_or_else(|| ScriptError::MissingField {
                            line,
                            block: "ORACLE".into(),
                            field: "router_ref".into(),
                        })?;
                self.kill_band_router_ref = Some(router_ref.clone());
            }
            other => {
                return Err(ScriptError::UnknownStatement {
                    line,
                    block: "ORACLE".into(),
                    statement: other.into(),
                });
            }
        }
        Ok(())
    }

    fn finish(self, line: usize) -> Result<OracleBlock, ScriptError> {
        Ok(OracleBlock {
            name: self.name,
            heartbeat_ms: self.heartbeat_ms.ok_or_else(|| ScriptError::MissingField {
                line,
                block: "ORACLE".into(),
                field: "HEARTBEAT".into(),
            })?,
            kill_band_router_ref: self.kill_band_router_ref,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SCRIPT: &str = r#"
ROUTER {
    COLLATERAL asset=USDC vault_cap=50000000
    PORTFOLIO_MARGIN model="cross_alpha" correl_matrix="router::correlations::v1"
    CAP_TTL ms=120000
    RESERVATION_BATCH ms=50
    CAP name="maker" asset=USDC limit=100000000 ttl_ms=60000
}

SLAB "perp:SOL-PERP" {
    MAKER_CLASS DLP allowance=5000000
    MATCHING fifo=true pending_promotion=true
    FEE maker_bps=2 taker_bps=5 rebate_delay_ms=50
    RISK imr_bps=500 mmr_bps=350
    ANTI_TOXICITY kill_band_bps=75 jit_penalty=true arg_tax_bps=10
    BATCH_WINDOW ms=48
    ORACLE_LINK id="pyth:SOLUSD"
}

ORACLE "pyth:SOLUSD" {
    HEARTBEAT ms=500
    KILL_BAND_SYNC router_ref="ROUTER"
}
"#;

    #[test]
    fn parses_router_slab_and_oracle() {
        let script = parse(SAMPLE_SCRIPT).expect("script should parse");
        assert_eq!(script.router.collateral_assets.len(), 1);
        assert_eq!(script.router.capabilities.len(), 1);
        assert_eq!(script.slabs.len(), 1);
        assert_eq!(script.slabs[0].oracle.as_deref(), Some("pyth:SOLUSD"));
        assert_eq!(script.oracles.len(), 1);
    }

    #[test]
    fn manifest_includes_descriptors_and_capabilities() {
        let script = parse(SAMPLE_SCRIPT).unwrap();
        let manifest = emit_manifest(&script);
        assert_eq!(manifest.router.capabilities.len(), 1);
        assert_eq!(manifest.router.capabilities[0].ttl_ms, 60000);
        assert_eq!(manifest.slabs[0].batch_window_ms, Some(48));
        let json = manifest_to_json(&manifest).unwrap();
        assert!(json.contains("reserve"));
    }

    #[test]
    fn rejects_missing_oracle_reference() {
        let script = r#"
ROUTER {
    COLLATERAL asset=USDC vault_cap=100
}
SLAB "perp:SOL-PERP" {
    MAKER_CLASS DLP allowance=10
    FEE maker_bps=1 taker_bps=1 rebate_delay_ms=1
    RISK imr_bps=1 mmr_bps=1
    ORACLE_LINK id="missing"
}
"#;
        let err = parse(script).unwrap_err();
        assert!(matches!(
            err,
            ScriptError::MissingField { .. }
                | ScriptError::MissingRouter
                | ScriptError::UnknownOracleReference { .. }
        ));
    }

    #[test]
    fn deterministic_ids_are_stable() {
        let id1 = route_id("router");
        let id2 = route_id("router");
        assert_eq!(id1, id2);
        let hold1 = hold_id("alice", "perp:SOL-PERP");
        let hold2 = hold_id("alice", "perp:SOL-PERP");
        assert_eq!(hold1, hold2);
    }
}
