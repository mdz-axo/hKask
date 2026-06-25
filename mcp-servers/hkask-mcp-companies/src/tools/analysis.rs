//! MAIA analysis and research tools.
use crate::*;

#[tool_router(router = analysis_router, vis = "pub")]
impl CompaniesServer {
    #[tool(
        description = "Analyze competitive moat using MAIA framework: gross margin stability and working capital market power signal"
    )]
    pub async fn moat_check(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "moat_check", async {
            validate_symbol(&symbol)?;

            // Fetch 10 years of key metrics for gross margin stability analysis
            let limit = "10";
            let metrics_result = self
                .fetch("key_metrics", &symbol, &[("limit", limit)])
                .await;

            let metrics = match metrics_result {
                Ok(v) => v,
                Err(e) => {
                    self.record_experience(
                        "moat_check",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                    return Err(e);
                }
            };

            let gross_margins = analysis::extract_gross_margins(&metrics);
            if gross_margins.is_empty() {
                let output = serde_json::json!({
                    "symbol": symbol,
                    "moat": "insufficient_data",
                    "reason": "No gross margin data available for this symbol",
                });
                self.record_experience(
                    "moat_check",
                    &format!("symbol={}", symbol),
                    "insufficient_data",
                    output.clone(),
                );
                return Ok(output);
            }

            let margin_values: Vec<f64> = gross_margins.iter().map(|(_, m)| *m).collect();
            let stability = analysis::gross_margin_stability(&margin_values);

            let wc_data = analysis::extract_wc_days(&metrics);
            let (wc_spread, dpo, dso) = match wc_data {
                Some((dpo_val, dso_val)) => (
                    analysis::working_capital_spread(dpo_val, dso_val),
                    Some(dpo_val),
                    Some(dso_val),
                ),
                None => (0.0, None, None),
            };

            let wc_label = analysis::wc_signal_label(wc_spread);
            let moat = analysis::classify_moat(stability, wc_spread, gross_margins.len());

            let output = serde_json::json!({
                "symbol": symbol,
                "moat": moat,
                "margin_stability": stability,
                "gross_margins": gross_margins,
                "working_capital": {
                    "spread_days": wc_spread,
                    "dpo": dpo,
                    "dso": dso,
                    "signal": wc_label,
                },
                "data_periods": gross_margins.len(),
            });
            self.record_experience(
                "moat_check",
                &format!("symbol={}", symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        })
        .await
    }

    #[tool(
        description = "CEO capital allocation scorecard (MAIA framework): rates how well management allocates capital by comparing returns on capital vs invested capital over time"
    )]
    pub async fn management_scorecard(
        &self,
        Parameters(SymbolRequest { symbol }): Parameters<SymbolRequest>,
    ) -> String {
        execute_tool(self, "management_scorecard", async {
            validate_symbol(&symbol)?;

            let limit = "10";
            let metrics_result = self.fetch(
     "key_metrics",
     &symbol,
     &[("limit", limit)],
 )
            .await;

            let bs_result = self.fetch(
     "balance_sheet",
     &symbol,
     &[("limit", limit)],
 )
            .await;

            let (metrics, balance_sheets) = match (metrics_result, bs_result) {
                (Ok(m), Ok(b)) => (m, b),
                (Err(e), _) | (_, Err(e)) => {
                    self.record_experience(
                        "management_scorecard",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                    return Err(e);
                }
            };

            let roic_values = analysis::extract_roic(&metrics);
            let capital_values = analysis::extract_invested_capital(&balance_sheets);

            // Align ROIC and invested capital by calendar year — they come from
            // different API endpoints and may have different year ranges.
            use std::collections::HashMap;
            let roic_by_year: HashMap<&str, f64> = roic_values
                .iter()
                .map(|(y, v)| (y.as_str(), *v))
                .collect();
            let mut aligned: Vec<(f64, f64)> = capital_values
                .iter()
                .filter_map(|(year, cap)| roic_by_year.get(year.as_str()).map(|r| (*r, *cap)))
                .collect();
            // Sort by invested capital ascending to preserve original ordering intent
            aligned.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let roic_nums: Vec<f64> = aligned.iter().map(|(r, _)| *r).collect();
            let capital_nums: Vec<f64> = aligned.iter().map(|(_, c)| *c).collect();

            let rating = analysis::ceo_capital_allocation_score(&roic_nums, &capital_nums);

            let output = serde_json::json!({
                "symbol": symbol,
                "ceo_rating": rating,
                "returns_on_capital": roic_values,
                "invested_capital": capital_values,
                "aligned_periods": aligned.len(),
                "data_periods": roic_nums.len(),
                "framework": "MAIA: Good = decreasing capital with improving returns, OR increasing capital with improving returns. Bad = increasing capital with decreasing returns.",
            });
            self.record_experience(
                "management_scorecard",
                &format!("symbol={}", symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    #[tool(
        description = "Working capital cycle analysis (MAIA CFO scorecard): tracks days payable, days sales outstanding, and cash conversion cycle over time"
    )]
    pub async fn working_capital_cycle(
        &self,
        Parameters(SymbolLimitRequest { symbol, limit }): Parameters<SymbolLimitRequest>,
    ) -> String {
        execute_tool(self, "working_capital_cycle", async {
            validate_symbol(&symbol)?;
            let limit_str = (limit.unwrap_or(10) as usize).min(40).to_string();

            let result = self.fetch(
     "key_metrics",
     &symbol,
     &[("limit", &limit_str)],
 )
            .await;

            let metrics = match result {
                Ok(v) => v,
                Err(e) => {
                    self.record_experience(
                        "working_capital_cycle",
                        &format!("symbol={}", symbol),
                        "error",
                        serde_json::json!({"error": e.to_json_string()}),
                    );
                    return Err(e);
                }
            };

            // Extract working capital days per period
            let arr = match metrics.as_array() {
                Some(a) => a,
                None => {
                    return Ok(serde_json::json!({"symbol": symbol, "error": "no data"}));
                }
            };

            let periods: Vec<serde_json::Value> = arr
                .iter()
                .filter_map(|entry| {
                    let year = entry.get("calendarYear")?.as_str().unwrap_or("");
                    let period = entry
                        .get("period")
                        .and_then(|p| p.as_str())
                        .unwrap_or("");
                    let dpo = entry.get("daysOfPayablesOutstanding")?.as_f64()?;
                    let dso = entry.get("daysOfSalesOutstanding")?.as_f64()?;
                    let dio = entry
                        .get("daysOfInventoryOutstanding")
                        .and_then(|v| v.as_f64());
                    let ccc = entry
                        .get("cashConversionCycle")
                        .and_then(|v| v.as_f64());
                    Some(serde_json::json!({
                        "year": year,
                        "period": period,
                        "dpo": dpo,
                        "dso": dso,
                        "dio": dio,
                        "spread": dpo - dso,
                        "cash_conversion_cycle": ccc,
                    }))
                })
                .collect();

            // MAIA CFO score: consistency of working capital management
            let spreads: Vec<f64> = periods
                .iter()
                .filter_map(|p| p.get("spread")?.as_f64())
                .collect();
            let spread_stability = analysis::gross_margin_stability(&spreads);

            let cfo_rating = if spread_stability > 0.8 {
                "stable"
            } else if spread_stability > 0.5 {
                "moderate"
            } else {
                "volatile"
            };

            let output = serde_json::json!({
                "symbol": symbol,
                "cfo_working_capital_rating": cfo_rating,
                "spread_stability": spread_stability,
                "periods": periods,
                "data_points": periods.len(),
                "framework": "MAIA CFO scorecard: stability of working capital management through economic conditions. The level is structural; consistency is management skill.",
            });
            self.record_experience(
                "working_capital_cycle",
                &format!("symbol={}", symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    #[tool(
        description = "Expectations gap: compare trailing 5-year actual performance to the future performance implied by the current price. Uses Gordon Growth Model to compute implied growth from valuation multiples vs historical profitability and growth."
    )]
    pub async fn expectations_gap(
        &self,
        Parameters(req): Parameters<ExpectationsGapRequest>,
    ) -> String {
        execute_tool(self, "expectations_gap", async {
            validate_symbol(&req.symbol)?;
            let target_return = req.target_return.unwrap_or(0.15);

            // Fetch 5 years of key metrics for historical profitability and growth
            let metrics_result = self.fetch(
     "key_metrics",
     &req.symbol,
     &[("limit", "5")],
 )
            .await;

            let profile_result = self.fetch(
     "company_profile",
     &req.symbol,
     &[],
 )
            .await;

            let bs_result = self.fetch(
     "balance_sheet",
     &req.symbol,
     &[("limit", "1")],
 )
            .await;

            let (metrics_arr, profile_arr, bs_arr) =
                match (metrics_result, profile_result, bs_result) {
                    (Ok(m), Ok(p), Ok(b)) => (m, p, b),
                    (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                        self.record_experience(
                            "expectations_gap",
                            &format!("symbol={}", req.symbol),
                            "error",
                            serde_json::json!({"error": e.to_json_string()}),
                        );
                        return Err(e);
                    }
                };

            // Extract trailing 5-year averages
            let metrics_list = metrics_arr.as_array();
            let profile = profile_arr.as_array().and_then(|a| a.first());
            let bs = bs_arr.as_array().and_then(|a| a.first());

            if metrics_list.is_none_or(|m| m.len() < 2) || profile.is_none() {
                return Ok(serde_json::json!({
                    "symbol": req.symbol,
                    "error": "insufficient data — need at least 2 years of metrics and a profile",
                }));
            }

            let metrics = metrics_list
                .as_ref()
                .expect("guarded by is_none_or check above");

            // ── Set A: Net Margins, Sales Growth, P/Sales ──
            let (net_margins, sales_growths) = extract_net_margin_and_sales_growth(metrics);
            let avg_net_margin: f64 = if !net_margins.is_empty() {
                net_margins.iter().sum::<f64>() / net_margins.len() as f64
            } else {
                0.0
            };
            let hist_sales_growth: f64 = if sales_growths.len() >= 2 {
                cagr_from_series(&sales_growths)
            } else {
                0.0
            };
            let ps = profile
                .and_then(|p| {
                    let price = p.get("price").and_then(|v| v.as_f64())?;
                    let sps = p.get("revenuePerShare").and_then(|v| v.as_f64())?;
                    if sps > 0.0 {
                        Some(price / sps)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            // ── Set B: ROE, Book Value Growth, P/Book ──
            let (roes, bv_growths) = extract_roe_and_bv_growth(metrics);
            let avg_roe: f64 = if !roes.is_empty() {
                roes.iter().sum::<f64>() / roes.len() as f64
            } else {
                0.0
            };
            let hist_bv_growth: f64 = if bv_growths.len() >= 2 {
                cagr_from_series(&bv_growths)
            } else {
                0.0
            };
            let pb = profile
                .and_then(|p| {
                    let price = p.get("price").and_then(|v| v.as_f64())?;
                    let bv = p.get("bookValuePerShare").and_then(|v| v.as_f64())?;
                    if bv > 0.0 {
                        Some(price / bv)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);

            // ── Set C: ROA, Asset Growth, P/Assets ──
            let (roas, asset_growths) = extract_roa_and_asset_growth(metrics);
            let avg_roa: f64 = if !roas.is_empty() {
                roas.iter().sum::<f64>() / roas.len() as f64
            } else {
                0.0
            };
            let hist_asset_growth: f64 = if asset_growths.len() >= 2 {
                cagr_from_series(&asset_growths)
            } else {
                0.0
            };
            let market_cap = profile
                .and_then(|p| p.get("mktCap").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let total_assets = bs
                .and_then(|b| b.get("totalAssets").and_then(|v| v.as_f64()))
                .unwrap_or(0.0);
            let pa = if total_assets > 0.0 {
                market_cap / total_assets
            } else {
                0.0
            };

            // ── Gordon Growth Model: implied growth from valuation ──
            // Assumes profitability and growth are correlated — a company expected
            // to grow 10% is also expected to improve profitability ~10%.
            // Total cash flow growth ≈ 2g, so: P/V = profitability / (r - 2g)
            // Rearranging: g = (r - profitability / (P/V)) / 2
            //
            // NaN → null in JSON to avoid silent corruption downstream.
            let implied_sales_growth = if ps > 0.0 && avg_net_margin > 0.0 {
                Some((target_return - avg_net_margin / ps) / 2.0)
            } else {
                None
            };
            let implied_bv_growth = if pb > 0.0 && avg_roe > 0.0 {
                Some((target_return - avg_roe / pb) / 2.0)
            } else {
                None
            };
            let implied_asset_growth = if pa > 0.0 && avg_roa > 0.0 {
                Some((target_return - avg_roa / pa) / 2.0)
            } else {
                None
            };

            // Helper: finite f64 → JSON number, else null
            let finite_or_null = |v: Option<f64>| -> serde_json::Value {
                match v {
                    Some(x) if x.is_finite() => serde_json::json!(x),
                    _ => serde_json::Value::Null,
                }
            };

            let output = serde_json::json!({
                "symbol": req.symbol,
                "target_return": target_return,
                "historical": {
                    "years": metrics.len(),
                    "set_a_income": {
                        "avg_net_margin": avg_net_margin,
                        "hist_sales_growth": hist_sales_growth,
                        "price_to_sales": ps,
                    },
                    "set_b_balance": {
                        "avg_roe": avg_roe,
                        "hist_book_value_growth": hist_bv_growth,
                        "price_to_book": pb,
                    },
                    "set_c_assets": {
                        "avg_roa": avg_roa,
                        "hist_asset_growth": hist_asset_growth,
                        "price_to_assets": pa,
                    },
                },
                "implied": {
                    "set_a_sales_growth_implied": finite_or_null(implied_sales_growth),
                    "set_b_book_value_growth_implied": finite_or_null(implied_bv_growth),
                    "set_c_asset_growth_implied": finite_or_null(implied_asset_growth),
                },
                "gaps": {
                    "sales_growth_gap": finite_or_null(implied_sales_growth.and_then(|i| if hist_sales_growth.is_finite() { Some(i - hist_sales_growth) } else { None })),
                    "book_value_growth_gap": finite_or_null(implied_bv_growth.and_then(|i| if hist_bv_growth.is_finite() { Some(i - hist_bv_growth) } else { None })),
                    "asset_growth_gap": finite_or_null(implied_asset_growth.and_then(|i| if hist_asset_growth.is_finite() { Some(i - hist_asset_growth) } else { None })),
                },
                "framework": "Gordon Growth Model with profitability-growth correlation: P/V = profitability / (r - 2g). Assumes growth and profitability improvement are proportional — a company expected to grow 10% is also expected to improve profitability ~10%. Total cash flow growth ≈ 2g. Implied g = (r - profitability / valuation_ratio) / 2. Compare to historical CAGR. Consistent methodology → rank ordering is accurate even if precise quantification is not.",
            });
            self.record_experience(
                "expectations_gap",
                &format!("symbol={}", req.symbol),
                "success",
                output.clone(),
            );
            Ok(output)
        }).await
    }

    #[tool(
        description = "Company screener. Parses natural language prompts into FMP stock screener API parameters. Supports filtering by market cap, price, volume, P/E ratio, dividend yield, beta, sector, industry, country, exchange, ROE, ROIC, and more. Use criteria_overrides to adjust parsed criteria. Reply with a modified prompt to refine results."
    )]
    pub async fn company_screener(
        &self,
        Parameters(req): Parameters<types::ScreenerRequest>,
    ) -> String {
        execute_tool(self, "company_screener", async {
            // Parse the prompt
            let mut criteria = screener::parse_screening_prompt(&req.prompt);

            // Apply user overrides
            if !req.criteria_overrides.is_null()
                && let Some(obj) = req.criteria_overrides.as_object()
                && let Some(crit_obj) = criteria.as_object_mut()
            {
                for (k, v) in obj {
                    crit_obj.insert(k.clone(), v.clone());
                }
            }

            // Add limit
            if let Some(obj) = criteria.as_object_mut() {
                obj.insert(
                    "limit".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(req.limit)),
                );
            }

            // Build query params from criteria
            let mut query_params: Vec<(&str, String)> = Vec::new();
            if let Some(obj) = criteria.as_object() {
                for (k, v) in obj {
                    if k != "apikey" {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        query_params.push((k.as_str(), val_str));
                    }
                }
            }

            // Call FMP screener API
            let url = "https://financialmodelingprep.com/api/v3/stock-screener";

            let response = self
                .client
                .get(url)
                .query(&[("apikey", self.fmp_api_key.as_str())])
                .query(
                    &query_params
                        .iter()
                        .map(|(k, v)| (*k, v.as_str()))
                        .collect::<Vec<_>>(),
                )
                .send()
                .await
                .map_err(|e| McpToolError::internal(e.to_string()))?;

            let body = response
                .text()
                .await
                .map_err(|e| McpToolError::internal(e.to_string()))?;

            let results: serde_json::Value =
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Array(vec![]));

            let count = results.as_array().map(|a| a.len()).unwrap_or(0);

            let output = serde_json::json!({
                "prompt": req.prompt,
                "criteria": criteria,
                "count": count,
                "results": results,
                "fibo": {
                    "screener": fibo::STOCK_SCREENER,
                    "market_capitalization": fibo::MARKET_CAPITALIZATION,
                    "price_earnings_ratio": fibo::PRICE_EARNINGS_RATIO,
                    "dividend_yield": fibo::DIVIDEND_YIELD,
                },
                "framework": "FMP Stock Screener. Parses natural language screening prompts into FMP screener API parameters. Use criteria_overrides to adjust parsed criteria. Reply with a modified prompt or criteria_overrides to refine results."
            });

            self.record_experience(
                "company_screener",
                &format!("prompt={}", &req.prompt[..req.prompt.len().min(80)]),
                "success",
                output.clone(),
            );
            Ok(output)
        })
        .await
    }

    #[tool(
        description = "Multi-provider fundamental research search. Searches across Exa, Tavily, and Brave for company-specific information and returns raw claims with source URLs. Use with thesis_test, scenario_weight, or guidance_check skills for structured financial analysis."
    )]
    pub async fn research_search(
        &self,
        Parameters(req): Parameters<types::ResearchSearchRequest>,
    ) -> String {
        execute_tool(self, "research_search", async {
            // 1. Fetch company profile for name
            let profile_result = self.fetch("company_profile", &req.symbol, &[]).await;
            let profile = profile_result?;
            let profile_obj = profile.as_array().and_then(|a| a.first());
            let company_name = profile_obj
                .and_then(|p| p.get("companyName").and_then(|v| v.as_str()))
                .unwrap_or(&req.symbol);

            // 2. Run multi-provider search
            let research = research::search_fundamental(
                &self.client,
                &req.symbol,
                company_name,
                &req.query,
                self.exa_api_key.as_deref(),
                self.tavily_api_key.as_deref(),
                self.brave_api_key.as_deref(),
            ).await;

            // 3. Build output
            let claims: Vec<serde_json::Value> = research.claims.iter().map(|c| {
                serde_json::json!({
                    "text": c.text,
                    "source": c.source,
                    "source_title": c.source_title,
                    "date": c.date,
                    "provider": c.provider,
                })
            }).collect();

            let output = serde_json::json!({
                "symbol": req.symbol,
                "query": req.query,
                "claims": claims,
                "claims_count": claims.len(),
                "providers": research.provider_summary.iter().map(|p| {
                    serde_json::json!({"provider": p.provider, "claims": p.claims_found, "status": p.status})
                }).collect::<Vec<_>>(),
                "fibo": {
                    "research_search": fibo::STOCK_SCREENER,
                },
                "framework": "Multi-provider fundamental research search (Exa, Tavily, Brave). Returns raw claims with source URLs. Use with thesis_test, scenario_weight, or guidance_check skills for structured financial analysis mapping claims to DCF assumptions."
            });

            self.record_experience("research_search", &format!("symbol={}", req.symbol), "success", output.clone());
            Ok(output)
        }).await
    }

    // ── Portfolio tools ──

}
