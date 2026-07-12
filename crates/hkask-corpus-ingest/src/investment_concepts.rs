//! Investment-domain concept registry for entity tagging.
//!
//! Concepts organized into tiers:
//! - **Tier 1 (Specific)**: Low-frequency, high-signal — narrow analytical terms
//! - **Tier 2 (Broad)**: Medium-frequency — domain-spanning bridge concepts
//! - **Tier 3 (Universal)**: High-frequency — ensure graph connectivity baseline
//!
//! The three-tier system ensures:
//! - Tier 1 terms create high-signal bridges between communities
//! - Tier 2 terms provide domain-level connectivity
//! - Tier 3 terms ensure every chunk has at least one tag (graph baseline)

/// Tier 1: Specific investment concepts — low frequency, high signal.
const TIER1_CONCEPTS: &[&str] = &[
    // Competitive Strategy (Greenwald)
    "competitive advantage",
    "barrier to entry",
    "switching cost",
    "customer captivity",
    "economies of scale",
    "economy of scale",
    "proprietary technology",
    "cost advantage",
    "network effect",
    "incumbent advantage",
    "entry barrier",
    "five forces",
    "bargaining power",
    "threat of entry",
    // Valuation (Damodaran, Fabozzi)
    "discounted cash flow",
    "residual income",
    "economic profit",
    "NOPAT",
    "terminal value",
    "intrinsic value",
    "enterprise value",
    "DuPont analysis",
    "reinvestment rate",
    "sustainable growth rate",
    // Value Investing (Klarman)
    "margin of safety",
    "Mr. Market",
    "circle of competence",
    // Systems (Meadows)
    "feedback loop",
    "reinforcing loop",
    "balancing loop",
    "leverage point",
    "systems thinking",
    "path dependence",
    // Mental Models (Parrish)
    "mental model",
    "first principles",
    "thought experiment",
    // Forecasting (Tetlock)
    "superforecasting",
    "outside view",
    "inside view",
    "Fermi estimate",
    "base rate",
    // Process (Rother)
    "improvement kata",
    "coaching kata",
    "target condition",
    "current condition",
    "scientific thinking",
    // Innovation
    "creative destruction",
    "disruptive innovation",
    "adoption curve",
    "S curve",
    // Risk
    "tail risk",
    "black swan",
    "real option",
    "scenario planning",
    "Monte Carlo",
];

/// Tier 2: Broad domain concepts — medium frequency, bridge categories.
const TIER2_CONCEPTS: &[&str] = &[
    // Finance & Valuation
    "return on capital",
    "return on equity",
    "return on assets",
    "cost of capital",
    "cost of equity",
    "free cash flow",
    "capital allocation",
    "capital structure",
    "capital expenditure",
    "working capital",
    "operating margin",
    "gross margin",
    "earnings yield",
    "dividend yield",
    "price to earnings",
    "price to book",
    "enterprise multiple",
    // Strategy
    "competitive position",
    "market share",
    "industry structure",
    "strategic advantage",
    "core competency",
    "vertical integration",
    "economies of scope",
    "learning curve",
    "brand equity",
    // Management
    "management quality",
    "corporate governance",
    "agency cost",
    "incentive alignment",
    "skin in the game",
    "executive compensation",
    // Behavioral
    "loss aversion",
    "confirmation bias",
    "overconfidence",
    "herd behavior",
    "anchoring",
    "endowment effect",
    "status quo bias",
    "hindsight bias",
    "framing effect",
    // Systems & Complexity
    "tipping point",
    "critical mass",
    "nonlinear",
    "emergence",
    "complex adaptive",
    "autocatalytic",
    "network effect",
    // Forecasting & Probability
    "base rate",
    "calibration",
    "Bayesian",
    "reference class",
    "regression to the mean",
    "survivorship bias",
    // Innovation & Technology
    "research and development",
    "technological change",
    "disruption",
    "innovation",
    "productivity growth",
    // Value Creation
    "value creation",
    "shareholder value",
    "value driver",
    "unit economics",
    "customer lifetime",
    "retention rate",
    "churn rate",
    "acquisition cost",
    // Risk
    "volatility",
    "uncertainty",
    "risk premium",
    "cost of debt",
    "credit rating",
    "default risk",
    "liquidity",
];

/// Tier 3: Universal bridge concepts — high frequency for graph connectivity.
const TIER3_CONCEPTS: &[&str] = &[
    // These are intentionally broad — every business/investment text uses them.
    // They ensure the graph has baseline connectivity while Tier 1/2 terms
    // create the meaningful bridges.
    "return",
    "profit",
    "margin",
    "growth",
    "valuation",
    "investor",
    "investment",
    "capital",
    "equity",
    "debt",
    "revenue",
    "earnings",
    "cash flow",
    "dividend",
    "buyback",
    "strategy",
    "competitive",
    "advantage",
    "industry",
    "sector",
    "economy",
    "management",
    "CEO",
    "board",
    "risk",
    "opportunity",
    "performance",
    "efficiency",
    "productivity",
];

/// Combined concept list for `tag_entities()`.
pub fn all_concepts() -> Vec<String> {
    [TIER1_CONCEPTS, TIER2_CONCEPTS, TIER3_CONCEPTS]
        .concat()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Key authors referenced across the corpus.
pub const INVESTMENT_AUTHORS: &[&str] = &[
    "Greenwald",
    "Damodaran",
    "Klarman",
    "Buffett",
    "Munger",
    "Graham",
    "Porter",
    "Tetlock",
    "Meadows",
    "Rother",
    "Fabozzi",
    "Parrish",
    "Kahneman",
    "Tversky",
    "Thaler",
    "Christensen",
    "Collins",
    "Taleb",
    "Mauboussin",
    "Penman",
    "Dodd",
    "Schumpeter",
    "Coase",
    "Williamson",
    "Kessler",
    "Norman",
    "Suzuki",
    "Bhide",
    "Reichheld",
    "McAfee",
    "Arthur",
    "Kersten",
    "Schwartz",
    "Beaubien",
];

/// Named analytical methods/frameworks.
pub const INVESTMENT_METHODS: &[&str] = &[
    "discounted cash flow",
    "residual income model",
    "comparable company",
    "precedent transaction",
    "competitive analysis",
    "moat analysis",
    "ROIC decomposition",
    "DuPont analysis",
    "scenario analysis",
    "sensitivity analysis",
    "Monte Carlo simulation",
    "PDCA cycle",
    "coaching kata",
    "improvement kata",
    "five forces",
    "SWOT analysis",
    "capital allocation framework",
    "expectations gap",
    "reverse DCF",
    "Fermi estimation",
    "outside view",
    "pre mortem",
];
