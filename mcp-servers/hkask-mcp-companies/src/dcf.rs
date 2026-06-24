//! Two-stage DCF + reverse DCF (Mauboussin expectations investing).

#[derive(Debug, Clone)]
pub struct DcfConfig {
    pub stage1_years: u8,
    pub stage2_years: u8,
    pub discount_rate: f64,
    pub terminal_growth: f64,
    pub terminal_method: TerminalMethod,
    pub frequency: ProjectionFrequency,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalMethod {
    Perpetuity,
    Multiple(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionFrequency {
    Annual,
    Quarterly,
}

impl Default for DcfConfig {
    fn default() -> Self {
        Self {
            stage1_years: 3,
            stage2_years: 7,
            discount_rate: 0.10,
            terminal_growth: 0.025,
            terminal_method: TerminalMethod::Perpetuity,
            frequency: ProjectionFrequency::Annual,
        }
    }
}

impl DcfConfig {
    pub fn total_years(&self) -> u8 {
        self.stage1_years + self.stage2_years
    }
    pub fn total_periods(&self) -> usize {
        match self.frequency {
            ProjectionFrequency::Annual => self.total_years() as usize,
            ProjectionFrequency::Quarterly => self.total_years() as usize * 4,
        }
    }
    pub fn stage1_periods(&self) -> usize {
        match self.frequency {
            ProjectionFrequency::Annual => self.stage1_years as usize,
            ProjectionFrequency::Quarterly => self.stage1_years as usize * 4,
        }
    }
    pub fn periods_per_year(&self) -> f64 {
        match self.frequency {
            ProjectionFrequency::Annual => 1.0,
            ProjectionFrequency::Quarterly => 4.0,
        }
    }
    pub fn period_discount_rate(&self) -> f64 {
        match self.frequency {
            ProjectionFrequency::Annual => self.discount_rate,
            ProjectionFrequency::Quarterly => (1.0 + self.discount_rate).powf(0.25) - 1.0,
        }
    }
    pub fn capped_terminal_growth(&self) -> f64 {
        self.terminal_growth.min(0.10)
    }
}

pub fn validate_dcf_config(config: &DcfConfig) -> Result<(), String> {
    if config.stage1_years < 1 || config.stage1_years > 3 {
        return Err("stage 1 must be 1-3 years".into());
    }
    if config.stage2_years < 2 || config.stage2_years > 7 {
        return Err("stage 2 must be 2-7 years".into());
    }
    if config.discount_rate <= 0.0 || config.discount_rate > 0.30 {
        return Err("discount rate must be 0%-30%".into());
    }
    if config.terminal_growth < 0.0 || config.terminal_growth > 0.10 {
        return Err("terminal growth rate must be 0%-10%".into());
    }
    if config.terminal_method == TerminalMethod::Perpetuity
        && config.discount_rate <= config.capped_terminal_growth()
    {
        return Err(format!(
            "discount rate ({:.1}%) must exceed terminal growth ({:.1}%)",
            config.discount_rate * 100.0,
            config.capped_terminal_growth() * 100.0
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct CompanyFundamentals {
    pub ttm_revenue: f64,
    pub ttm_fcf: f64,
    pub fcf_margin: f64,
    pub hist_revenue_growth: f64,
    pub shares_outstanding: f64,
    pub current_price: f64,
    pub market_cap: f64,
}

#[derive(Debug, Clone)]
pub struct ProjectedPeriod {
    pub period: usize,
    pub year: f64,
    pub revenue: f64,
    pub fcf: f64,
    pub growth_rate: f64,
    pub discount_factor: f64,
    pub present_value: f64,
}

#[derive(Debug, Clone)]
pub struct DcfResult {
    pub config: DcfConfig,
    pub periods: Vec<ProjectedPeriod>,
    pub sum_pv_cash_flows: f64,
    pub terminal_value: f64,
    pub terminal_pv: f64,
    pub enterprise_value: f64,
    pub equity_value: f64,
    pub intrinsic_per_share: f64,
    pub current_price: f64,
    pub margin_of_safety: f64,
}

pub fn run_dcf(
    fundamentals: &CompanyFundamentals,
    config: &DcfConfig,
) -> Result<DcfResult, String> {
    validate_dcf_config(config)?;
    let stage1_start_growth = fundamentals.hist_revenue_growth;
    let terminal_g = config.capped_terminal_growth();
    let stage1_periods = config.stage1_periods();
    let stage2_periods = config.total_periods() - stage1_periods;
    let total_periods = config.total_periods();
    let ppy = config.periods_per_year();
    let period_rate = config.period_discount_rate();
    let stage1_growth_mid = (stage1_start_growth + terminal_g) / 2.0;

    let mut periods = Vec::with_capacity(total_periods);
    let mut revenue = fundamentals.ttm_revenue;

    for p in 0..stage1_periods {
        let progress = if stage1_periods > 1 {
            p as f64 / (stage1_periods - 1) as f64
        } else {
            0.0
        };
        let growth = stage1_start_growth + (stage1_growth_mid - stage1_start_growth) * progress;
        revenue *= 1.0 + growth / ppy;
        let fcf = revenue * fundamentals.fcf_margin;
        let df = 1.0 / (1.0 + period_rate).powi((p + 1) as i32);
        periods.push(ProjectedPeriod {
            period: p,
            year: (p + 1) as f64 / ppy,
            revenue,
            fcf,
            growth_rate: growth,
            discount_factor: df,
            present_value: fcf * df,
        });
    }

    let stage1_end_growth = periods
        .last()
        .map(|pp| pp.growth_rate)
        .unwrap_or(stage1_growth_mid);

    for p in 0..stage2_periods {
        let global_p = stage1_periods + p;
        let progress = if stage2_periods > 1 {
            p as f64 / (stage2_periods - 1) as f64
        } else {
            0.0
        };
        let growth = stage1_end_growth + (terminal_g - stage1_end_growth) * progress;
        revenue *= 1.0 + growth / ppy;
        let fcf = revenue * fundamentals.fcf_margin;
        let df = 1.0 / (1.0 + period_rate).powi((global_p + 1) as i32);
        periods.push(ProjectedPeriod {
            period: global_p,
            year: (global_p + 1) as f64 / ppy,
            revenue,
            fcf,
            growth_rate: growth,
            discount_factor: df,
            present_value: fcf * df,
        });
    }

    let sum_pv_cash_flows: f64 = periods.iter().map(|p| p.present_value).sum();
    let last_fcf = periods.last().map(|p| p.fcf).unwrap_or(0.0);
    let terminal_value = match config.terminal_method {
        TerminalMethod::Perpetuity => {
            let tg = terminal_g.min(config.discount_rate - 0.005);
            last_fcf * (1.0 + tg) / (config.discount_rate - tg)
        }
        TerminalMethod::Multiple(multiple) => last_fcf * multiple,
    };
    let terminal_df = 1.0 / (1.0 + period_rate).powi(total_periods as i32);
    let terminal_pv = terminal_value * terminal_df;
    let enterprise_value = sum_pv_cash_flows + terminal_pv;
    let intrinsic_per_share = if fundamentals.shares_outstanding > 0.0 {
        enterprise_value / fundamentals.shares_outstanding
    } else {
        0.0
    };
    let margin_of_safety = if fundamentals.current_price > 0.0 {
        (intrinsic_per_share - fundamentals.current_price) / fundamentals.current_price
    } else {
        0.0
    };

    Ok(DcfResult {
        config: config.clone(),
        periods,
        sum_pv_cash_flows,
        terminal_value,
        terminal_pv,
        enterprise_value,
        equity_value: enterprise_value,
        intrinsic_per_share,
        current_price: fundamentals.current_price,
        margin_of_safety,
    })
}

pub fn reverse_dcf(
    fundamentals: &CompanyFundamentals,
    config: &DcfConfig,
) -> Result<(f64, DcfResult), String> {
    validate_dcf_config(config)?;
    let target_price = fundamentals.current_price;
    if target_price <= 0.0 {
        return Err("current price must be positive for reverse DCF".into());
    }

    let mut lo = -0.50f64;
    let mut hi = 1.00f64;

    let r_lo = run_dcf(
        &CompanyFundamentals {
            hist_revenue_growth: lo,
            ..fundamentals.clone()
        },
        config,
    )?;
    if r_lo.intrinsic_per_share > target_price {
        return Err(format!(
            "price ({:.2}) below intrinsic at {:.0}% growth",
            target_price,
            lo * 100.0
        ));
    }
    let r_hi = run_dcf(
        &CompanyFundamentals {
            hist_revenue_growth: hi,
            ..fundamentals.clone()
        },
        config,
    )?;
    if r_hi.intrinsic_per_share < target_price {
        return Err(format!(
            "price ({:.2}) implies growth > {:.0}%",
            target_price,
            hi * 100.0
        ));
    }

    for _ in 0..50 {
        let mid = (lo + hi) / 2.0;
        if (hi - lo).abs() < 0.0001 {
            let r = run_dcf(
                &CompanyFundamentals {
                    hist_revenue_growth: mid,
                    ..fundamentals.clone()
                },
                config,
            )?;
            return Ok((mid, r));
        }
        let result = run_dcf(
            &CompanyFundamentals {
                hist_revenue_growth: mid,
                ..fundamentals.clone()
            },
            config,
        )?;
        if result.intrinsic_per_share > target_price {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mid = (lo + hi) / 2.0;
    let r = run_dcf(
        &CompanyFundamentals {
            hist_revenue_growth: mid,
            ..fundamentals.clone()
        },
        config,
    )?;
    Ok((mid, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CompanyFundamentals {
        CompanyFundamentals {
            ttm_revenue: 100_000.0,
            ttm_fcf: 15_000.0,
            fcf_margin: 0.15,
            hist_revenue_growth: 0.08,
            shares_outstanding: 1_000.0,
            current_price: 150.0,
            market_cap: 150_000.0,
        }
    }

    #[test]
    fn dcf_ok() {
        assert!(validate_dcf_config(&DcfConfig::default()).is_ok());
    }
    #[test]
    fn dcf_bad_stage1() {
        let mut c = DcfConfig::default();
        c.stage1_years = 0;
        assert!(validate_dcf_config(&c).is_err());
        c.stage1_years = 4;
        assert!(validate_dcf_config(&c).is_err());
    }
    #[test]
    fn dcf_bad_stage2() {
        let mut c = DcfConfig::default();
        c.stage2_years = 1;
        assert!(validate_dcf_config(&c).is_err());
        c.stage2_years = 8;
        assert!(validate_dcf_config(&c).is_err());
    }
    #[test]
    fn dcf_bad_terminal() {
        let mut c = DcfConfig::default();
        c.discount_rate = 0.08;
        c.terminal_growth = 0.09;
        assert!(validate_dcf_config(&c).is_err());
    }
    #[test]
    fn dcf_run() {
        let r = run_dcf(&sample(), &DcfConfig::default()).unwrap();
        assert_eq!(r.periods.len(), 10);
        assert!(r.sum_pv_cash_flows > 0.0 && r.terminal_pv > 0.0 && r.intrinsic_per_share > 0.0);
    }
    #[test]
    fn dcf_quarterly() {
        let mut c = DcfConfig::default();
        c.frequency = ProjectionFrequency::Quarterly;
        assert_eq!(run_dcf(&sample(), &c).unwrap().periods.len(), 40);
    }
    #[test]
    fn dcf_stage1_gt_stage2() {
        let r = run_dcf(&sample(), &DcfConfig::default()).unwrap();
        let s1: Vec<f64> = r.periods[..3].iter().map(|p| p.growth_rate).collect();
        let s2: Vec<f64> = r.periods[3..].iter().map(|p| p.growth_rate).collect();
        assert!(s1.iter().sum::<f64>() / 3.0 >= s2.iter().sum::<f64>() / 7.0);
    }
    #[test]
    fn dcf_multiple() {
        let mut c = DcfConfig::default();
        c.terminal_method = TerminalMethod::Multiple(15.0);
        let r = run_dcf(&sample(), &c).unwrap();
        assert!(r.terminal_value > 0.0);
    }
    #[test]
    fn reverse_dcf_bounds_err() {
        let f = CompanyFundamentals {
            ttm_revenue: 100.0,
            ttm_fcf: 2.0,
            fcf_margin: 0.02,
            hist_revenue_growth: 0.02,
            shares_outstanding: 10.0,
            current_price: 500.0,
            market_cap: 5000.0,
        };
        assert!(reverse_dcf(&f, &DcfConfig::default()).is_err());
    }
}
