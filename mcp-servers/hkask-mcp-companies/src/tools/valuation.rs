//! Valuation and forecasting tools.
use crate::*;

#[tool_router(router = valuation_router, vis = "pub")]
impl CompaniesServer {
    #[tool(
        description = "Comparable company analysis. Gathers valuation multiples (P/E, P/B, P/S, EV/EBITDA) from peer companies in the same industry, alongside a DCF intrinsic value overlay for the target. Multiples provide market-relative context; DCF provides fundamentals-anchored valuation. Accepts optional comma-separated peer list."
    )]
    pub async fn comparable_analysis(
        &self,
        Parameters(req): Parameters<types::ComparableAnalysisRequest>,
    ) -> String {
        execute_tool(self, "comparable_analysis", async {
            validate_symbol(&req.symbol)?;

            // 1. Fetch target company profile and key_metrics
            let profile_result = self
                .fetch("company_profile", &req.symbol, &[])
                .await;
            let metrics_result = self
                .fetch("key_metrics", &req.symbol, &[("limit", "1")])
                .await;

            let (profile, metrics) = match (profile_result, metrics_result) {
                (Ok(p), Ok(m)) => (p, m),
                (Err(e), _) | (_, Err(e)) => return Err(e),
            };

            let profile_arr = profile.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile_arr.and_then(|a| a.first());
            let metrics_obj = metrics_arr.and_then(|a| a.first());

            let Some(profile_data) = profile_obj else {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "company profile not found"}));
            };

            // 2. Parse peers (comma-separated)
            let peers: Vec<String> = req
                .peers
                .as_ref()
                .map(|s| {
                    s.split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // 3. Fetch peer profiles and metrics in parallel
            let mut peer_data: Vec<(String, serde_json::Value, Option<serde_json::Value>)> =
                Vec::new();
            for peer_sym in &peers {
                let pp_result = self.fetch("company_profile", peer_sym, &[]).await;
                let pm_result = self
                    .fetch("key_metrics", peer_sym, &[("limit", "1")])
                    .await;
                let pp = pp_result.unwrap_or(serde_json::Value::Null);
                let pm =
                    pm_result
                        .ok()
                        .and_then(|v| v.as_array().and_then(|a| a.first().cloned()));
                peer_data.push((peer_sym.clone(), pp, pm));
            }

            // 4. Build comparison table
            fn build_row(
                sym: &str,
                profile: &serde_json::Value,
                metrics: Option<&serde_json::Value>,
            ) -> serde_json::Value {
                let name = profile
                    .get("companyName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let price = profile.get("price").and_then(|v| v.as_f64());
                let mkt_cap = profile.get("mktCap").and_then(|v| v.as_f64());
                let pe = metrics.and_then(|m| m.get("peRatio").and_then(|v| v.as_f64()));
                let pb = metrics.and_then(|m| {
                    m.get("priceToBookRatio").and_then(|v| v.as_f64())
                });
                let ps = metrics.and_then(|m| {
                    m.get("priceToSalesRatio").and_then(|v| v.as_f64())
                });
                let ev_ebitda = metrics.and_then(|m| {
                    m.get("evToEbitda")
                        .or_else(|| m.get("enterpriseValueMultiple"))
                        .and_then(|v| v.as_f64())
                });
                let div_yield =
                    metrics.and_then(|m| m.get("dividendYield").and_then(|v| v.as_f64()));
                let rev_growth =
                    metrics.and_then(|m| m.get("revenueGrowth").and_then(|v| v.as_f64()));

                let mut row = serde_json::json!({
                    "symbol": sym,
                    "name": name,
                });
                if let Some(v) = price {
                    row["price"] = serde_json::json!(v);
                }
                if let Some(v) = mkt_cap {
                    row["market_cap"] = serde_json::json!(v);
                }
                if let Some(v) = pe {
                    row["pe_ratio"] = serde_json::json!(v);
                }
                if let Some(v) = pb {
                    row["price_to_book"] = serde_json::json!(v);
                }
                if let Some(v) = ps {
                    row["price_to_sales"] = serde_json::json!(v);
                }
                if let Some(v) = ev_ebitda {
                    row["ev_to_ebitda"] = serde_json::json!(v);
                }
                if let Some(v) = div_yield {
                    row["dividend_yield"] = serde_json::json!(v);
                }
                if let Some(v) = rev_growth {
                    row["revenue_growth"] = serde_json::json!(v);
                }
                row
            }

            let mut comparison = vec![build_row(&req.symbol, profile_data, metrics_obj)];
            for (sym, pp, pm) in &peer_data {
                comparison.push(build_row(sym, pp, pm.as_ref()));
            }

            // 5. DCF overlay on target
            let dcf_overlay = {
                let inc_res = self
                    .fetch("income_statement", &req.symbol, &[("limit", "5")])
                    .await;
                let bal_res = self
                    .fetch("balance_sheet", &req.symbol, &[("limit", "5")])
                    .await;
                let cf_res = self
                    .fetch("cash_flow_statement", &req.symbol, &[("limit", "5")])
                    .await;
                let km_res = self
                    .fetch("key_metrics", &req.symbol, &[("limit", "5")])
                    .await;

                match (inc_res, bal_res, cf_res, km_res) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(km)) => {
                        let income_arr = inc.as_array();
                        let balance_arr = bal.as_array();
                        let cf_arr = cf.as_array();
                        let metrics_arr = km.as_array();

                        if income_arr.is_none_or(|a| a.is_empty())
                            || balance_arr.is_none_or(|a| a.is_empty())
                            || cf_arr.is_none_or(|a| a.is_empty())
                        {
                            serde_json::json!({"error": "insufficient data for DCF"})
                        } else {
                            let income_data = income_arr.unwrap();
                            let balance_data = balance_arr.unwrap();
                            let cf_data = cf_arr.unwrap();
                            let metrics_data: &[serde_json::Value] =
                                metrics_arr.map_or(&[], |v| v);

                            let hist = financial_model::HistoricalSnapshot::from_api_json(
                                income_data,
                                balance_data,
                                cf_data,
                                metrics_data,
                                profile_data,
                            );

                            if hist.revenue.len() < 2 {
                                serde_json::json!({"error": "insufficient historical data"})
                            } else {
                                let mut assumptions =
                                    financial_model::ProjectionAssumptions::from_history(&hist);
                                if let Some(dr) = req.discount_rate {
                                    assumptions.discount_rate = dr;
                                }
                                if let Some(tg) = req.terminal_growth {
                                    assumptions.terminal_growth = tg;
                                }
                                let current_price = profile_data
                                    .get("price")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let model = financial_model::project_model(
                                    &hist,
                                    &assumptions,
                                    current_price,
                                );
                                let margin_of_safety =
                                    if current_price > 0.0 {
                                        (model.intrinsic_per_share - current_price)
                                            / current_price
                                    } else {
                                        0.0
                                    };
                                serde_json::json!({
                                    "intrinsic_per_share": model.intrinsic_per_share,
                                    "current_price": current_price,
                                    "margin_of_safety": margin_of_safety,
                                })
                            }
                        }
                    }
                    _ => serde_json::json!({"error": "DCF overlay unavailable"}),
                }
            };

            let company_name = profile_data
                .get("companyName")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let sector = profile_data
                .get("sector")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let industry = profile_data
                .get("industry")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let output = serde_json::json!({
                "symbol": req.symbol,
                "company_name": company_name,
                "sector": sector,
                "industry": industry,
                "peers": peers,
                "dcf_overlay": dcf_overlay,
                "comparison": comparison,
                "fibo": {
                    "comparable_analysis": fibo::COMPARABLE_COMPANY_ANALYSIS,
                    "pe_ratio": fibo::PRICE_EARNINGS_RATIO,
                    "price_to_book": fibo::PRICE_TO_BOOK_RATIO,
                    "price_to_sales": fibo::PRICE_TO_SALES_RATIO,
                    "ev_to_ebitda": fibo::ENTERPRISE_VALUE_MULTIPLE,
                    "dividend_yield": fibo::DIVIDEND_YIELD,
                    "revenue_growth": fibo::REVENUE_GROWTH_RATE,
                },
                "framework": "Comparable company analysis. Valuation multiples (P/E, P/B, P/S) from peer companies alongside DCF intrinsic value. Multiples provide market-relative context; DCF provides fundamentals-anchored valuation.",
            });

            self.record_experience(
                "comparable_analysis",
                &format!("symbol={}", req.symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        })
        .await
    }

    #[tool(
        description = "Tornado chart sensitivity analysis. Varies each DCF driver (revenue growth, gross margin, D&A, capex, NWC, discount rate) by +/- range_pct (default 10%) while holding others constant. Returns drivers ranked by impact on intrinsic value per share. Identifies which assumptions most affect the valuation."
    )]
    pub async fn sensitivity_analysis(
        &self,
        Parameters(req): Parameters<types::SensitivityAnalysisRequest>,
    ) -> String {
        execute_tool(self, "sensitivity_analysis", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;

            let (income, balance, cf, metrics, profile) =
                match (income_result, balance_result, cf_result, metrics_result, profile_result) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(m), Ok(p)) => (inc, bal, cf, m, p),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => {
                        return Err(e);
                    }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }
            if let Some(rg) = req.revenue_growth {
                assumptions.revenue_growth = rg;
            }
            if let Some(gm) = req.gross_margin {
                assumptions.gross_margin = gm;
            }
            if let Some(da) = req.da_to_revenue {
                assumptions.da_to_revenue = da;
            }
            if let Some(cx) = req.capex_to_revenue {
                assumptions.capex_to_revenue = cx;
            }
            if let Some(nw) = req.nwc_to_revenue {
                assumptions.nwc_to_revenue = nw;
            }
            if let Some(tr) = req.tax_rate {
                assumptions.tax_rate = tr;
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let base_model = financial_model::project_model(&hist, &assumptions, current_price);
            let base_intrinsic = base_model.intrinsic_per_share;

            let sensitivity_results =
                financial_model::sensitivity_analysis(&hist, &assumptions, req.range_pct);

            let drivers: Vec<serde_json::Value> = sensitivity_results.iter().map(|r| {
                serde_json::json!({
                    "driver": r.driver,
                    "label": r.label,
                    "base_value": r.base_value,
                    "low_value": r.low_value,
                    "high_value": r.high_value,
                    "intrinsic_low": r.intrinsic_low,
                    "intrinsic_high": r.intrinsic_high,
                    "delta_pct": r.delta_pct,
                    "fibo": r.fibo_concept,
                })
            }).collect();

            let mut fibo_map = serde_json::Map::new();
            fibo_map.insert(
                "sensitivity_analysis".to_string(),
                serde_json::Value::String(fibo::SENSITIVITY_ANALYSIS.to_string()),
            );
            for r in &sensitivity_results {
                fibo_map.insert(
                    r.driver.clone(),
                    serde_json::Value::String(r.fibo_concept.to_string()),
                );
            }

            let output = serde_json::json!({
                "symbol": req.symbol,
                "base_intrinsic": base_intrinsic,
                "current_price": current_price,
                "range_pct": req.range_pct,
                "drivers": drivers,
                "fibo": fibo_map,
                "framework": "Tornado chart sensitivity analysis. Varies each DCF driver by +/- range_pct while holding others constant. Drivers ranked by impact on intrinsic value per share. Identifies which assumptions most affect the valuation.",
            });

            self.record_experience("sensitivity_analysis", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Monte Carlo DCF simulation. Runs N simulations (default 1000, clamped 100-10000) with each DCF assumption randomized uniformly within its +/- configured range. Returns intrinsic value distribution (percentiles p10/p25/median/p75/p90, histogram), probability of undervaluation, and base case comparison. Quantifies valuation uncertainty from assumption ranges."
    )]
    pub async fn monte_carlo_dcf(
        &self,
        Parameters(req): Parameters<types::MonteCarloDcfRequest>,
    ) -> String {
        execute_tool(self, "monte_carlo_dcf", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;

            let (income, balance, cf, metrics, profile) =
                match (income_result, balance_result, cf_result, metrics_result, profile_result) {
                    (Ok(inc), Ok(bal), Ok(cf), Ok(m), Ok(p)) => (inc, bal, cf, m, p),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => {
                        return Err(e);
                    }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }
            if let Some(rg) = req.revenue_growth {
                assumptions.revenue_growth = rg;
            }
            if let Some(gm) = req.gross_margin {
                assumptions.gross_margin = gm;
            }
            if let Some(da) = req.da_to_revenue {
                assumptions.da_to_revenue = da;
            }
            if let Some(cx) = req.capex_to_revenue {
                assumptions.capex_to_revenue = cx;
            }
            if let Some(nw) = req.nwc_to_revenue {
                assumptions.nwc_to_revenue = nw;
            }
            if let Some(tr) = req.tax_rate {
                assumptions.tax_rate = tr;
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let ranges = financial_model::McRange {
                revenue_growth: req.range_revenue_growth,
                gross_margin: req.range_gross_margin,
                da_to_revenue: req.range_da,
                capex_to_revenue: req.range_capex,
                nwc_to_revenue: req.range_nwc,
                discount_rate: req.range_discount_rate,
            };

            let mut rng = rand::rng();
            let sims = req.simulations.clamp(100, 10_000) as usize;
            let result = financial_model::monte_carlo_dcf(&hist, &assumptions, sims, &ranges, current_price, &mut rng);

            let histogram: Vec<serde_json::Value> = result.histogram.iter().map(|(bucket, count)| {
                serde_json::json!({"bucket": bucket, "count": count})
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "current_price": current_price,
                "simulations": result.simulations,
                "distribution": {
                    "base_intrinsic": result.base_intrinsic,
                    "mean": result.mean_intrinsic,
                    "std_dev": result.std_dev,
                    "min": result.min_intrinsic,
                    "p10": result.p10,
                    "p25": result.p25,
                    "median": result.median,
                    "p75": result.p75,
                    "p90": result.p90,
                    "max": result.max_intrinsic,
                    "prob_undervalued": result.prob_undervalued,
                    "histogram": histogram,
                },
                "fibo": {
                    "monte_carlo": fibo::MONTE_CARLO_DCF,
                    "probability_undervalued": fibo::PROBABILITY_OF_UNDERVALUATION
                },
                "framework": "Monte Carlo DCF. Runs N simulations with each assumption sampled uniformly within +/- configured ranges. Produces intrinsic value distribution (percentiles), probability of undervaluation, and histogram. Quantifies valuation uncertainty from assumption ranges."
            });

            self.record_experience("monte_carlo_dcf", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Calibrated superforecast. Runs Fermi decomposition on growth and margin estimates, applies outside view (base rate) and inside view adjustments, then distributes probabilities across the four Schwartz scenarios. Produces a probability-weighted intrinsic value and compares it to the market price. Anchored to Tetlock's GJP methodology. Collaborative — you provide base rates and reference counts; the tool computes calibrations."
    )]
    pub async fn calibrate_forecast(
        &self,
        Parameters(req): Parameters<types::CalibrateForecastRequest>,
    ) -> String {
        execute_tool(self, "calibrate_forecast", async {
            validate_symbol(&req.symbol)?;

            let income_result = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
            let balance_result = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
            let metrics_result = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;
            let cf_result = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;

            let (income, balance, metrics, profile, cf) =
                match (income_result, balance_result, metrics_result, profile_result, cf_result) {
                    (Ok(inc), Ok(bal), Ok(m), Ok(p), Ok(c)) => (inc, bal, m, p, c),
                    (Err(e), _, _, _, _)
                    | (_, Err(e), _, _, _)
                    | (_, _, Err(e), _, _)
                    | (_, _, _, Err(e), _)
                    | (_, _, _, _, Err(e)) => { return Err(e); }
                };

            let income_arr = income.as_array();
            let balance_arr = balance.as_array();
            let cf_arr = cf.as_array();
            let metrics_arr = metrics.as_array();
            let profile_obj = profile.as_array().and_then(|a| a.first());

            if income_arr.is_none_or(|a| a.is_empty())
                || balance_arr.is_none_or(|a| a.is_empty())
                || cf_arr.is_none_or(|a| a.is_empty())
                || profile_obj.is_none()
            {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient data"}));
            }

            let income_data = income_arr.unwrap();
            let balance_data = balance_arr.unwrap();
            let cf_data = cf_arr.unwrap();
            let metrics_data: &[serde_json::Value] = metrics_arr.map_or(&[], |v| v);
            let profile_data = profile_obj.unwrap();

            let hist = financial_model::HistoricalSnapshot::from_api_json(
                income_data, balance_data, cf_data, metrics_data, profile_data,
            );

            if hist.revenue.len() < 2 {
                return Ok(serde_json::json!({"symbol": req.symbol, "error": "insufficient historical data — need at least 2 years of revenue"}));
            }

            let current_price = profile_data.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let hist_revenue_growth = hist.revenue_cagr();

            // Build projection assumptions from history
            let mut assumptions = financial_model::ProjectionAssumptions::from_history(&hist);
            let stage1 = req.stage1_years.unwrap_or(3);
            let stage2 = req.stage2_years.unwrap_or(7);
            assumptions.stage1_years = stage1;
            assumptions.total_years = stage1 + stage2;
            if let Some(dr) = req.discount_rate {
                assumptions.discount_rate = dr;
            }
            if let Some(tg) = req.terminal_growth {
                assumptions.terminal_growth = tg;
            }

            // Run scenarios
            let matrix = scenarios::ScenarioMatrix::growth_x_margin(hist_revenue_growth, assumptions.gross_margin);
            let results = scenarios::run_scenario_analysis(&hist, &assumptions, &matrix);

            // Build Fermi estimates from server-level defaults, apply user overrides
            let mut growth_fermi = self.fermi_defaults.growth_questions.clone();
            let mut margin_fermi = self.fermi_defaults.margin_questions.clone();

            if !req.growth_fermi_overrides.is_empty() {
                let o: Vec<(usize, f64, f64)> = req.growth_fermi_overrides.iter()
                    .map(|ov| (ov.index, ov.estimate, ov.confidence)).collect();
                superforecast::apply_fermi_overrides(&mut growth_fermi, &o);
            }
            if !req.margin_fermi_overrides.is_empty() {
                let o: Vec<(usize, f64, f64)> = req.margin_fermi_overrides.iter()
                    .map(|ov| (ov.index, ov.estimate, ov.confidence)).collect();
                superforecast::apply_fermi_overrides(&mut margin_fermi, &o);
            }

            let growth_inside = req.growth_estimate.unwrap_or_else(|| {
                superforecast::calibrate_from_fermi(&growth_fermi)
            });
            let margin_inside = req.margin_estimate.unwrap_or_else(|| {
                superforecast::calibrate_from_fermi(&margin_fermi)
            });

            let ref_class = req.reference_class.unwrap_or_else(|| "S&P 500 large-cap, 2015-2025".into());
            let ref_count = req.reference_count.unwrap_or(500);

            let (growth_calibrated, growth_conf) = superforecast::outside_view_adjustment(
                0.55, growth_inside, ref_count,
            );
            let (margin_calibrated, margin_conf) = superforecast::outside_view_adjustment(
                0.50, margin_inside, ref_count,
            );

            // Distribute probabilities across scenarios
            let weighted = superforecast::distribute_scenario_probabilities(
                growth_calibrated, margin_calibrated, &results,
            );
            let expected_value = superforecast::expected_intrinsic(&weighted);
            let market_gap = if current_price > 0.0 { (expected_value - current_price) / current_price } else { 0.0 };

            // Generate forecast ID and store calibrated projection for later decomposition
            let forecast_id = Uuid::new_v4().to_string();
            {
                // Apply calibrated estimates to assumptions for the stored model
                assumptions.revenue_growth = growth_calibrated;
                assumptions.gross_margin = margin_calibrated;
                let model = financial_model::project_model(&hist, &assumptions, current_price);
                let stored = StoredForecast {
                    forecast_id: forecast_id.clone(),
                    symbol: req.symbol.clone(),
                    created_at: now_rfc3339(),
                    model,
                    assumptions: assumptions.clone(),
                    hist: hist.clone(),
                    current_price,
                    intrinsic_per_share: expected_value,
                };
                let mut store = self.forecast_store.lock().unwrap();
                store.insert(forecast_id.clone(), stored);
            }

            let fermi_output: Vec<serde_json::Value> = growth_fermi.iter().zip(margin_fermi.iter()).map(|(g, m)| {
                serde_json::json!({
                    "growth_sub_q": g.question, "growth_estimate": g.estimate, "growth_confidence": g.confidence,
                    "margin_sub_q": m.question, "margin_estimate": m.estimate, "margin_confidence": m.confidence,
                })
            }).collect();

            let scenario_output: Vec<serde_json::Value> = weighted.iter().map(|w| {
                serde_json::json!({"name": w.name, "intrinsic": w.intrinsic_per_share, "probability": w.probability})
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "forecast_id": forecast_id,
                "current_price": current_price,
                "calibration": {
                    "growth": {"inside_estimate": growth_inside, "calibrated": growth_calibrated, "confidence": growth_conf},
                    "margin": {"inside_estimate": margin_inside, "calibrated": margin_calibrated, "confidence": margin_conf},
                    "reference_class": ref_class,
                    "reference_count": ref_count,
                    "method": "Fermi decomposition + outside/inside view calibration",
                },
                "fermi_decomposition": fermi_output,
                "scenarios": scenario_output,
                "expected_intrinsic": expected_value,
                "market_gap_pct": market_gap,
                "interpretation": if market_gap > 0.10 { "significantly_undervalued" } else if market_gap > 0.0 { "modestly_undervalued" } else if market_gap > -0.10 { "fairly_valued" } else { "overvalued" },
                "framework": "Tetlock GJP Superforecasting pipeline: Fermi decomposition → outside/inside view calibration → Bayesian-ready probability estimates → scenario-weighted intrinsic value. Probabilities are probability-weighted scenario intrinsic values compared to market price. Brier score tracking available when outcomes are recorded via result_feedback.",
            });

            self.record_experience("calibrate_forecast", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Record a forecast outcome to close the superforecasting loop. Forecast a valuation multiple and price change over a horizon (3mo/6mo/1yr/2yr/3yr), then record what actually happened. Computes Brier scores on multiple direction and price return vs a tolerance band. When forecast_id is provided (from dcf_valuation or calibrate_forecast), looks up the stored 11-line-item projection model and decomposes the return gap into revenue growth, gross margin, D&A, capex, NWC, multiple expansion, and net debt contributions."
    )]
    pub async fn forecast_record(
        &self,
        Parameters(req): Parameters<types::ForecastRecordRequest>,
    ) -> String {
        execute_tool(self, "forecast_record", async {
            validate_symbol(&req.symbol)?;

            // Validate horizon
            if !superforecast::FORECAST_HORIZONS.contains(&req.horizon.as_str()) {
                return Err(McpToolError::invalid_argument(format!(
                    "horizon must be one of: {}", superforecast::FORECAST_HORIZONS.join(", ")
                )));
            }

            // Brier scores on binary outcomes
            // Multiple: was actual multiple >= forecast? (binary direction)
            let multiple_higher = req.actual_multiple >= req.forecast_multiple;
            let p_multiple_up = 0.5;
            let multiple_brier = superforecast::brier_score(p_multiple_up, multiple_higher);

            // Price change: was actual return within 20% tolerance of forecast?
            let return_accurate = superforecast::within_tolerance(
                req.forecast_price_change, req.actual_price_change, 0.20,
            );
            let return_brier = superforecast::brier_score(0.7, return_accurate);

            let combined = (multiple_brier + return_brier) / 2.0;

            // Gap decomposition: use stored forecast model if available
            let mut decomposition: Option<serde_json::Value> = None;
            if let Some(ref forecast_id) = req.forecast_id {
                // Clone stored forecast out of lock scope before async fetches
                let stored_opt = {
                    let store = self.forecast_store.lock().unwrap();
                    store.get(forecast_id).cloned()
                };
                if let Some(stored) = stored_opt {
                    // Fetch actual financials at the outcome date for decomposition
                    let actual_income = self.fetch("income_statement", &req.symbol, &[("limit", "5")]).await;
                    let actual_balance = self.fetch("balance_sheet", &req.symbol, &[("limit", "5")]).await;
                    let actual_cf = self.fetch("cash_flow_statement", &req.symbol, &[("limit", "5")]).await;
                    let actual_metrics = self.fetch("key_metrics", &req.symbol, &[("limit", "5")]).await;
                    let actual_profile = self.fetch("company_profile", &req.symbol, &[]).await;

                    if let (Ok(inc), Ok(bal), Ok(cf), Ok(metrics), Ok(prof)) =
                        (&actual_income, &actual_balance, &actual_cf, &actual_metrics, &actual_profile)
                    {
                        let inc_arr = inc.as_array();
                        let bal_arr = bal.as_array();
                        let cf_arr = cf.as_array();
                        let met_arr = metrics.as_array();
                        let prof_obj = prof.as_array().and_then(|a| a.first());

                        if inc_arr.is_some_and(|a| !a.is_empty())
                            && bal_arr.is_some_and(|a| !a.is_empty())
                            && cf_arr.is_some_and(|a| !a.is_empty())
                        {
                            let actual_hist = financial_model::HistoricalSnapshot::from_api_json(
                                inc_arr.unwrap(),
                                bal_arr.unwrap(),
                                cf_arr.unwrap(),
                                met_arr.map_or(&[] as &[serde_json::Value], |v| v),
                                prof_obj.unwrap_or(&serde_json::Value::Null),
                            );

                            // Run decomposition
                            let gap = financial_model::decompose_gap(
                                &stored.model,
                                &stored.assumptions,
                                &actual_hist,
                                current_price_from_multiple(req.actual_multiple, &actual_hist),
                                req.actual_multiple,
                                stored.intrinsic_per_share,
                                stored.current_price,
                            );

                            decomposition = Some(serde_json::json!({
                                "total_return_gap": gap.total_return_gap,
                                "components": {
                                    "revenue_growth": {
                                        "contribution": gap.revenue_growth_contribution,
                                        "projected_growth": stored.assumptions.revenue_growth,
                                        "actual_growth": actual_hist.revenue_cagr(),
                                    },
                                    "gross_margin": {
                                        "contribution": gap.gross_margin_contribution,
                                        "projected": stored.assumptions.gross_margin,
                                        "actual": actual_hist.gross_margin(),
                                    },
                                    "da": {
                                        "contribution": gap.da_contribution,
                                        "projected": stored.assumptions.da_to_revenue,
                                        "actual": actual_hist.da_to_revenue(),
                                    },
                                    "capex": {
                                        "contribution": gap.capex_contribution,
                                        "projected": stored.assumptions.capex_to_revenue,
                                        "actual": actual_hist.capex_to_revenue(),
                                    },
                                    "nwc": {
                                        "contribution": gap.nwc_contribution,
                                        "projected": stored.assumptions.nwc_to_revenue,
                                        "actual": actual_hist.nwc_to_revenue(),
                                    },
                                    "multiple": {
                                        "contribution": gap.multiple_contribution,
                                        "projected": projected_terminal_multiple(&stored.model),
                                        "actual": req.actual_multiple,
                                    },
                                    "net_debt": {
                                        "contribution": gap.net_debt_contribution,
                                        "projected": stored.model.net_debt,
                                        "actual": actual_hist.net_debt(),
                                    },
                                },
                                "residual": gap.residual,
                            }));
                        }
                    }
                }
            }

            // Legacy gap narrative (used when no forecast_id or decomposition fails)
            let multiple_gap = req.actual_multiple - req.forecast_multiple;
            let return_gap = req.actual_price_change - req.forecast_price_change;
            let gap_narrative = if decomposition.is_some() {
                "full_decomposition"
            } else if multiple_gap.abs() > 2.0 && return_gap.abs() > 0.05 {
                "multiple_and_return_diverged"
            } else if multiple_gap.abs() > 2.0 {
                "multiple_drove_gap"
            } else if return_gap.abs() > 0.05 {
                "return_drove_gap"
            } else {
                "forecast_accurate"
            };

            // Store in daemon
            if let Some(ref daemon) = self.daemon {
                let mut value = serde_json::json!({
                    "symbol": req.symbol,
                    "forecast_date": req.forecast_date,
                    "horizon": req.horizon,
                    "forecast_multiple": req.forecast_multiple,
                    "forecast_price_change": req.forecast_price_change,
                    "outcome_date": req.outcome_date,
                    "actual_multiple": req.actual_multiple,
                    "actual_price_change": req.actual_price_change,
                    "multiple_brier": multiple_brier,
                    "return_brier": return_brier,
                    "combined_brier": combined,
                    "timestamp": now_rfc3339(),
                });
                if let Some(ref dec) = decomposition {
                    value["decomposition"] = dec.clone();
                }
                if let Some(ref fid) = req.forecast_id {
                    value["forecast_id"] = serde_json::Value::String(fid.clone());
                }
                let daemon_clone = daemon.clone();
                let replicant = self.replicant.clone();
                let symbol = req.symbol.clone();
                #[allow(clippy::let_underscore_future)]
                let _ = tokio::spawn(async move {
                    let _ = daemon_clone.store_experience(
                        &replicant, &format!("forecast_outcome:{symbol}"), "outcome_recorded",
                        &value, Some(0.95),
                    ).await;
                });
            }

            let mut output = serde_json::json!({
                "status": "recorded",
                "symbol": req.symbol,
                "horizon": req.horizon,
                "forecast": {
                    "multiple": req.forecast_multiple,
                    "price_change_pct": req.forecast_price_change,
                },
                "actual": {
                    "multiple": req.actual_multiple,
                    "price_change_pct": req.actual_price_change,
                },
                "gaps": {
                    "multiple_gap": multiple_gap,
                    "return_gap": return_gap,
                    "narrative": gap_narrative,
                },
                "brier": {
                    "multiple_direction": multiple_brier,
                    "return_accuracy": return_brier,
                    "combined": combined,
                    "interpretation": superforecast::brier_interpretation(combined),
                },
                "framework": "Forecast-Record-Score (Tetlock GJP). Brier scores on binary outcomes: multiple direction and return accuracy within 20% tolerance. When forecast_id is provided, runs full 11-line-item decomposition (revenue growth, gross margin, D&A, capex, NWC, multiple, net debt).",
            });

            if let Some(dec) = decomposition {
                output["decomposition"] = dec;
            }
            if let Some(ref fid) = req.forecast_id {
                output["forecast_id"] = serde_json::Value::String(fid.clone());
            }

            self.record_experience("forecast_record", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    #[tool(
        description = "Rate a previous tool result on a 1–5 scale with optional comments. Score: 5 = exceeded expectations, 3 = met expectations, 1 = completely missed. Both score and comments are optional — provide either, both, or neither to acknowledge you saw the result. Feeds the learning loop."
    )]
    pub async fn result_feedback(
        &self,
        Parameters(types::ResultFeedbackRequest {
            tool,
            query,
            score,
            comments,
        }): Parameters<types::ResultFeedbackRequest>,
    ) -> String {
        execute_tool(self, "result_feedback", async {
            // Validate score range if provided
            if let Some(s) = score
                && !(1..=5).contains(&s)
            {
                return Err(McpToolError::invalid_argument(format!(
                    "score must be 1–5, got {s}"
                )));
            }

            // Accept empty feedback as an acknowledgment (no score, no comments = "I saw it")
            let has_feedback = score.is_some() || !comments.is_empty();

            // Store feedback as a daemon experience linked to the original tool.
            if let Some(ref daemon) = self.daemon {
                let value = serde_json::json!({
                    "tool": tool,
                    "query": query,
                    "score": score,
                    "comments": comments,
                    "has_feedback": has_feedback,
                    "timestamp": now_rfc3339(),
                });
                let daemon_clone = daemon.clone();
                let replicant = self.replicant.clone();
                let tool_for_spawn = tool.clone();
                tokio::spawn(async move {
                    let _ = daemon_clone
                        .store_experience(
                            &replicant,
                            &format!("feedback:{tool_for_spawn}"),
                            "user_rated",
                            &value,
                            Some(0.95),
                        )
                        .await;
                });
            }

            // Kanban-style learning: feedback updates in-process state.
            // Extracts symbol from query to track per-symbol provider quality.
            if let Some(sym) = parse_symbol_from_query(&query)
                && let Ok(mut state) = self.learning.lock()
            {
                let prov = if comments.contains("provider=eodhd") {
                    "EODHD"
                } else if comments.contains("provider=fmp") {
                    "FMP"
                } else if sym.contains('.') {
                    "EODHD"
                } else {
                    "FMP"
                };
                state.record(&sym, prov, score);
            }

            let summary = if has_feedback {
                if let Some(s) = score {
                    format!("score {s}/5")
                } else {
                    "comments only".to_string()
                }
            } else {
                "acknowledged".to_string()
            };

            Ok(serde_json::json!({
                "status": "recorded",
                "tool": tool,
                "query": query,
                "summary": summary,
            }))
        })
        .await
    }
}
