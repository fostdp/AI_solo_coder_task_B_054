use crate::config::PenetrationConfig;

#[derive(Clone)]
pub struct PegPenetrationService {
    config: PenetrationConfig,
}

impl PegPenetrationService {
    pub fn new(config: PenetrationConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PenetrationConfig {
        &self.config
    }

    pub fn predict(
        &self,
        time_hours: f64,
        pressure_diff: Option<f64>,
        viscosity: Option<f64>,
        num_points: Option<usize>,
    ) -> (Vec<f64>, Vec<f64>, f64) {
        let n = num_points.unwrap_or(self.config.default_num_points);
        let n = n.max(2);
        let mut time_points = Vec::with_capacity(n);
        let mut depth_values = Vec::with_capacity(n);

        let k = self.config.default_permeability;
        let mu = viscosity.unwrap_or(self.config.default_viscosity);
        let phi = self.config.default_porosity;
        let delta_p = pressure_diff.unwrap_or(self.config.default_pressure_diff);
        let sample_len = self.config.sample_length;

        for i in 0..n {
            let t_hours = (time_hours * i as f64) / (n - 1) as f64;
            let t_seconds = t_hours * 3600.0;

            let depth = ((2.0 * k * delta_p * t_seconds) / (mu * phi)).sqrt();
            let depth_mm = depth * 1000.0;

            time_points.push(t_hours);
            depth_values.push(depth_mm.min(sample_len * 1000.0));
        }

        let final_depth = ((2.0 * k * delta_p * time_hours * 3600.0) / (mu * phi)).sqrt() * 1000.0;

        (time_points, depth_values, final_depth.min(sample_len * 1000.0))
    }

    pub fn flow_rate(&self, pressure_diff: f64, area: f64, length: f64) -> f64 {
        let k = self.config.default_permeability;
        let mu = self.config.default_viscosity;
        (k * area * pressure_diff) / (mu * length)
    }

    pub fn penetration_velocity(&self, pressure_diff: f64, current_depth: f64) -> f64 {
        if current_depth <= 0.0 {
            return f64::INFINITY;
        }
        let k = self.config.default_permeability;
        let mu = self.config.default_viscosity;
        let phi = self.config.default_porosity;
        (k * pressure_diff) / (mu * phi * current_depth)
    }

    pub fn permeability(&self) -> f64 {
        self.config.default_permeability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PenetrationConfig {
        PenetrationConfig {
            default_permeability: 1e-14,
            default_porosity: 0.4,
            default_viscosity: 0.056,
            default_pressure_diff: 101325.0,
            sample_length: 0.5,
            default_num_points: 50,
        }
    }

    #[test]
    fn test_darcy_law() {
        let service = PegPenetrationService::new(test_config());
        let (times, depths, final_depth) = service.predict(48.0, Some(101325.0), Some(0.056), Some(50));

        assert_eq!(times.len(), 50);
        assert_eq!(depths.len(), 50);
        assert!(depths[0] < depths[depths.len() - 1]);
        assert!(final_depth > 0.0);
    }

    #[test]
    fn test_penetration_increases_with_time() {
        let service = PegPenetrationService::new(test_config());
        let (_, depths1, _) = service.predict(24.0, None, None, Some(2));
        let (_, depths2, _) = service.predict(48.0, None, None, Some(2));
        assert!(depths2[1] > depths1[1], "Longer time should give deeper penetration");
    }

    #[test]
    fn test_flow_rate_positive() {
        let service = PegPenetrationService::new(test_config());
        let rate = service.flow_rate(101325.0, 0.01, 0.05);
        assert!(rate > 0.0);
    }
}
