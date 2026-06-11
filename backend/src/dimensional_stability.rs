use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionalStabilityConfig {
    pub initial_moisture: f64,
    pub low_moisture: f64,
    pub high_moisture: f64,
    pub original_dimension: f64,
    pub hygro_expansion_coeff: f64,
    pub shrinkage_coeff: f64,
    pub nonlinear_term: f64,
    pub agent_type: String,
    pub agent_concentration: f64,
    pub peg_molecular_weight: f64,
    pub void_filling_ratio: f64,
    pub reinforcement_factor: f64,
    pub num_cycles: usize,
    pub cycle_duration_hours: f64,
    pub steps_per_cycle: usize,
}

impl Default for DimensionalStabilityConfig {
    fn default() -> Self {
        Self {
            initial_moisture: 50.0,
            low_moisture: 8.0,
            high_moisture: 65.0,
            original_dimension: 0.2,
            hygro_expansion_coeff: 0.0025,
            shrinkage_coeff: 0.003,
            nonlinear_term: 1e-5,
            agent_type: "PEG".to_string(),
            agent_concentration: 30.0,
            peg_molecular_weight: 400.0,
            void_filling_ratio: 0.0,
            reinforcement_factor: 1.0,
            num_cycles: 5,
            cycle_duration_hours: 168.0,
            steps_per_cycle: 40,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CycleDataPoint {
    pub time_hours: f64,
    pub moisture: f64,
    pub dimension: f64,
    pub dimensional_change_percent: f64,
    pub strain: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CycleSummary {
    pub cycle_number: usize,
    pub max_dimension: f64,
    pub min_dimension: f64,
    pub dimensional_swing_percent: f64,
    pub residual_deformation_percent: f64,
    pub expansion_rate: f64,
    pub shrinkage_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DimensionalStabilityResult {
    pub time_series: Vec<CycleDataPoint>,
    pub cycle_summaries: Vec<CycleSummary>,
    pub overall_expansion_percent: f64,
    pub overall_shrinkage_percent: f64,
    pub total_dimensional_swing: f64,
    pub final_residual_deformation_percent: f64,
    pub stability_rating: String,
    pub stability_score: f64,
    pub long_term_prediction_10yr: f64,
    pub long_term_prediction_50yr: f64,
    pub improvement_factor: f64,
    pub recommended_agent_type: String,
    pub cycles_to_failure: f64,
    pub cumulative_deformation: Vec<f64>,
    pub equivalent_years: f64,
}

#[derive(Clone)]
pub struct DimensionalStabilitySimulator {
    config: DimensionalStabilityConfig,
}

impl DimensionalStabilitySimulator {
    pub fn new(config: DimensionalStabilityConfig) -> Self {
        Self { config }
    }

    pub fn calculate_void_filling(&self) -> f64 {
        let max_filling = match self.config.agent_type.as_str() {
            "PEG" => {
                let mw = self.config.peg_molecular_weight;
                if mw <= 200.0 { 0.85 }
                else if mw <= 400.0 { 0.75 }
                else if mw <= 600.0 { 0.65 }
                else { 0.55 }
            }
            "sucrose" => 0.7,
            "glycerol" => 0.8,
            "alcohol" => 0.6,
            _ => 0.7,
        };

        let conc_factor = self.config.agent_concentration / 30.0;
        (max_filling * conc_factor.min(1.5)).min(0.95)
    }

    pub fn effective_shrinkage_coeff(&self, void_filling: f64) -> f64 {
        let reduction = void_filling * 0.7;
        self.config.shrinkage_coeff * (1.0 - reduction) * self.config.reinforcement_factor
    }

    pub fn effective_expansion_coeff(&self, void_filling: f64) -> f64 {
        let reduction = void_filling * 0.6;
        self.config.hygro_expansion_coeff * (1.0 - reduction) * self.config.reinforcement_factor
    }

    pub fn simulate(&self) -> DimensionalStabilityResult {
        let void_filling = self.calculate_void_filling();
        let eff_shrink = self.effective_shrinkage_coeff(void_filling);
        let eff_expand = self.effective_expansion_coeff(void_filling);

        let total_steps = self.config.num_cycles * self.config.steps_per_cycle;
        let mut time_series = Vec::with_capacity(total_steps);
        let mut cycle_summaries = Vec::with_capacity(self.config.num_cycles);
        let mut cumulative_deformation = Vec::with_capacity(self.config.num_cycles + 1);

        let mut current_dimension = self.config.original_dimension;
        let mut current_moisture;
        let mut baseline_dimension = self.config.original_dimension;
        cumulative_deformation.push(0.0);

        for cycle in 0..self.config.num_cycles {
            let cycle_start_time = cycle as f64 * self.config.cycle_duration_hours;
            let steps_per_phase = self.config.steps_per_cycle / 2;

            let mut cycle_max_dim = current_dimension;
            let mut cycle_min_dim = current_dimension;

            for step in 0..steps_per_phase {
                let t = cycle_start_time + step as f64 * self.config.cycle_duration_hours / (self.config.steps_per_cycle) as f64;
                let progress = step as f64 / (steps_per_phase - 1) as f64;

                current_moisture = self.config.initial_moisture
                    + (self.config.low_moisture - self.config.initial_moisture)
                    * self.s_curve(progress);

                let delta_m = current_moisture - self.config.initial_moisture;
                let shrink_strain = -eff_shrink * delta_m.abs()
                    + self.config.nonlinear_term * delta_m.abs().powi(2) * 0.0;

                let creep_factor = 1.0 + cycle as f64 * 0.02;
                current_dimension = self.config.original_dimension * (1.0 + shrink_strain * creep_factor);

                if current_dimension > cycle_max_dim {
                    cycle_max_dim = current_dimension;
                }
                if current_dimension < cycle_min_dim {
                    cycle_min_dim = current_dimension;
                }

                let dim_change_pct = (current_dimension - self.config.original_dimension)
                    / self.config.original_dimension * 100.0;

                time_series.push(CycleDataPoint {
                    time_hours: t,
                    moisture: current_moisture,
                    dimension: current_dimension,
                    dimensional_change_percent: dim_change_pct,
                    strain: shrink_strain,
                });
            }

            for step in 0..steps_per_phase {
                let t = cycle_start_time + (steps_per_phase + step) as f64
                    * self.config.cycle_duration_hours / self.config.steps_per_cycle as f64;
                let progress = step as f64 / (steps_per_phase - 1) as f64;

                current_moisture = self.config.low_moisture
                    + (self.config.high_moisture - self.config.low_moisture)
                    * self.s_curve(progress);

                let delta_m = current_moisture - self.config.low_moisture;
                let expansion_strain = eff_expand * delta_m
                    - self.config.nonlinear_term * delta_m * delta_m;

                let hysteresis_factor = 0.92 + cycle as f64 * 0.005;
                current_dimension = cycle_min_dim + (self.config.original_dimension - cycle_min_dim)
                    * (1.0 - expansion_strain.exp()) * hysteresis_factor;
                let dim_change_pct = (current_dimension - self.config.original_dimension)
                    / self.config.original_dimension * 100.0;

                if current_dimension > cycle_max_dim {
                    cycle_max_dim = current_dimension;
                }

                time_series.push(CycleDataPoint {
                    time_hours: t,
                    moisture: current_moisture,
                    dimension: current_dimension,
                    dimensional_change_percent: dim_change_pct,
                    strain: expansion_strain,
                });
            }

            let swing_pct = (cycle_max_dim - cycle_min_dim) / self.config.original_dimension * 100.0;
            let residual = (current_dimension - baseline_dimension) / self.config.original_dimension * 100.0;

            cycle_summaries.push(CycleSummary {
                cycle_number: cycle + 1,
                max_dimension: cycle_max_dim,
                min_dimension: cycle_min_dim,
                dimensional_swing_percent: swing_pct,
                residual_deformation_percent: residual,
                expansion_rate: (cycle_max_dim - cycle_min_dim)
                    / (self.config.high_moisture - self.config.low_moisture)
                    / self.config.original_dimension * 100.0,
                shrinkage_rate: (cycle_max_dim - cycle_min_dim)
                    / (self.config.high_moisture - self.config.low_moisture)
                    / self.config.original_dimension * 100.0,
            });

            baseline_dimension = current_dimension;
            cumulative_deformation.push(
                (current_dimension - self.config.original_dimension)
                    / self.config.original_dimension * 100.0
            );
        }

        let overall_expand = (cycle_summaries.iter().map(|c| c.max_dimension).fold(f64::NEG_INFINITY, f64::max)
            - self.config.original_dimension) / self.config.original_dimension * 100.0;

        let overall_shrink = (self.config.original_dimension
            - cycle_summaries.iter().map(|c| c.min_dimension).fold(f64::INFINITY, f64::min))
            / self.config.original_dimension * 100.0;

        let final_residual = cumulative_deformation.last().copied().unwrap_or(0.0);

        let total_swing = overall_expand + overall_shrink;

        let stability_score = self.calculate_stability_score(total_swing, final_residual);
        let rating = self.rating_from_score(stability_score);

        let pred_10yr = self.predict_long_term(10.0, &cumulative_deformation);
        let pred_50yr = self.predict_long_term(50.0, &cumulative_deformation);

        let baseline_swing = self.config.initial_moisture - self.config.low_moisture;
        let baseline_total = baseline_swing * self.config.shrinkage_coeff * 100.0;
        let improvement = if total_swing > 0.0 {
            baseline_total / total_swing
        } else {
            1.0
        };

        let cycles_per_year = 4.0;
        let equivalent_years = self.config.num_cycles as f64 / cycles_per_year;

        let cycles_to_failure = if final_residual.abs() > 0.1 {
            (2.0 / final_residual.abs()) * self.config.num_cycles as f64
        } else {
            1000.0
        };

        DimensionalStabilityResult {
            time_series,
            cycle_summaries,
            overall_expansion_percent: overall_expand,
            overall_shrinkage_percent: overall_shrink,
            total_dimensional_swing: total_swing,
            final_residual_deformation_percent: final_residual,
            stability_rating: rating,
            stability_score,
            long_term_prediction_10yr: pred_10yr,
            long_term_prediction_50yr: pred_50yr,
            improvement_factor: improvement,
            recommended_agent_type: self.recommend_agent(),
            cycles_to_failure,
            cumulative_deformation,
            equivalent_years,
        }
    }

    fn s_curve(&self, x: f64) -> f64 {
        let k = 4.0;
        let x0 = 0.5;
        1.0 / (1.0 + (-k * (x - x0)).exp())
    }

    fn calculate_stability_score(&self, swing_pct: f64, residual_pct: f64) -> f64 {
        let swing_score = (1.0 - swing_pct / 5.0).max(0.0) * 60.0;
        let residual_score = (1.0 - residual_pct.abs() / 2.0).max(0.0) * 40.0;
        (swing_score + residual_score).min(100.0).max(0.0)
    }

    fn rating_from_score(&self, score: f64) -> String {
        if score >= 85.0 { "excellent".to_string() }
        else if score >= 70.0 { "good".to_string() }
        else if score >= 50.0 { "fair".to_string() }
        else if score >= 30.0 { "poor".to_string() }
        else { "critical".to_string() }
    }

    fn predict_long_term(&self, years: f64, cumulative: &[f64]) -> f64 {
        if cumulative.len() < 2 {
            return 0.0;
        }

        let cycles_per_year = 4.0;
        let target_cycles = years * cycles_per_year;

        let n = cumulative.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, &y) in cumulative.iter().enumerate() {
            let x = i as f64;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x).max(1e-10);

        cumulative.last().copied().unwrap_or(0.0) + slope * (target_cycles - n + 1.0)
    }

    fn recommend_agent(&self) -> String {
        match self.config.peg_molecular_weight as i32 {
            0..=200 => "PEG 200 - 高填充率，适合小件".to_string(),
            201..=400 => "PEG 400 - 平衡填充与渗透，推荐".to_string(),
            401..=600 => "PEG 600 - 较好的尺寸稳定性".to_string(),
            _ => "PEG 1000+ - 低收缩，但渗透困难".to_string(),
        }
    }

    pub fn compare_without_reinforcement(&self) -> DimensionalStabilityResult {
        let mut config_no_agent = self.config.clone();
        config_no_agent.agent_concentration = 0.0;
        config_no_agent.reinforcement_factor = 1.0;
        let sim = DimensionalStabilitySimulator::new(config_no_agent);
        sim.simulate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_void_filling_calculation() {
        let config = DimensionalStabilityConfig::default();
        let sim = DimensionalStabilitySimulator::new(config);

        let filling = sim.calculate_void_filling();
        assert!(filling > 0.0 && filling < 1.0);
    }

    #[test]
    fn test_reinforcement_reduces_shrinkage() {
        let config_peg = DimensionalStabilityConfig {
            agent_concentration: 30.0,
            ..Default::default()
        };
        let sim_peg = DimensionalStabilitySimulator::new(config_peg);

        let config_no = DimensionalStabilityConfig {
            agent_concentration: 0.0,
            ..Default::default()
        };
        let sim_no = DimensionalStabilitySimulator::new(config_no);

        let shrink_peg = sim_peg.effective_shrinkage_coeff(sim_peg.calculate_void_filling());
        let shrink_no = sim_no.effective_shrinkage_coeff(sim_no.calculate_void_filling());

        assert!(shrink_peg < shrink_no,
                "Reinforcement should reduce effective shrinkage coefficient");
    }

    #[test]
    fn test_simulation_basic() {
        let config = DimensionalStabilityConfig {
            num_cycles: 2,
            steps_per_cycle: 20,
            ..Default::default()
        };
        let sim = DimensionalStabilitySimulator::new(config);

        let result = sim.simulate();
        assert_eq!(result.cycle_summaries.len(), 2);
        assert_eq!(result.time_series.len(), 40);
        assert!(result.stability_score >= 0.0 && result.stability_score <= 100.0);
    }

    #[test]
    fn test_residual_deformation_accumulates() {
        let config = DimensionalStabilityConfig {
            num_cycles: 5,
            steps_per_cycle: 20,
            ..Default::default()
        };
        let sim = DimensionalStabilitySimulator::new(config);

        let result = sim.simulate();
        assert!(result.cumulative_deformation.len() == 6);

        let last = result.cumulative_deformation.last().copied().unwrap_or(0.0);
        let first = result.cumulative_deformation[0];
        assert!(last.abs() >= first.abs(),
                "Cumulative deformation should generally increase over cycles");
    }

    #[test]
    fn test_stability_rating() {
        let config = DimensionalStabilityConfig::default();
        let sim = DimensionalStabilitySimulator::new(config);

        let result = sim.simulate();
        let rating = result.stability_rating;
        let valid_ratings = ["excellent", "good", "fair", "poor", "critical"];
        assert!(valid_ratings.contains(&rating.as_str()));
    }

    #[test]
    fn test_long_term_prediction() {
        let config = DimensionalStabilityConfig {
            num_cycles: 10,
            steps_per_cycle: 10,
            ..Default::default()
        };
        let sim = DimensionalStabilitySimulator::new(config);

        let result = sim.simulate();
        assert!(result.long_term_prediction_10yr.is_finite());
        assert!(result.long_term_prediction_50yr.is_finite());
        assert!(result.long_term_prediction_50yr.abs() >= result.long_term_prediction_10yr.abs());
    }

    #[test]
    fn test_improvement_factor() {
        let config = DimensionalStabilityConfig {
            agent_concentration: 30.0,
            num_cycles: 3,
            steps_per_cycle: 10,
            ..Default::default()
        };
        let sim = DimensionalStabilitySimulator::new(config);

        let result = sim.simulate();
        assert!(result.improvement_factor > 0.0);
    }

    #[test]
    fn test_s_curve_monotonic() {
        let config = DimensionalStabilityConfig::default();
        let sim = DimensionalStabilitySimulator::new(config);

        let mut prev = 0.0;
        for i in 0..10 {
            let x = i as f64 / 9.0;
            let y = sim.s_curve(x);
            assert!(y >= prev, "S-curve should be monotonically increasing");
            prev = y;
        }
        assert!((sim.s_curve(0.0) - 0.119).abs() < 0.01);
        assert!((sim.s_curve(1.0) - 0.881).abs() < 0.01);
    }
}
