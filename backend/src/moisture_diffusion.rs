use std::f64::consts::PI;
use crate::config::DiffusionConfig;

#[derive(Clone)]
pub struct MoistureDiffusionService {
    config: DiffusionConfig,
}

impl MoistureDiffusionService {
    pub fn new(config: DiffusionConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &DiffusionConfig {
        &self.config
    }

    pub fn with_crusting(mut self, alpha: f64, threshold: f64) -> Self {
        self.config.crusting_alpha = alpha;
        self.config.crust_threshold = threshold;
        self
    }

    fn effective_diffusion_coefficient(&self, d0: f64, current_concentration: f64, initial_concentration: f64) -> f64 {
        let c_ratio = current_concentration / initial_concentration.max(1.0);
        if current_concentration < self.config.crust_threshold {
            let crust_factor = (current_concentration / self.config.crust_threshold).max(0.05);
            d0 * crust_factor.powf(self.config.crusting_alpha)
        } else {
            d0 * (1.0 - 0.3 * (1.0 - c_ratio))
        }
    }

    pub fn predict_loss(
        &self,
        initial_moisture: f64,
        target_moisture: f64,
        time_hours: f64,
        num_points: Option<usize>,
    ) -> (Vec<f64>, Vec<f64>, f64) {
        let n = num_points.unwrap_or(self.config.default_num_points).max(2);
        let l = self.config.thickness.max(1e-10);
        let c0 = initial_moisture;
        let ce = target_moisture;
        let num_spatial = 21;
        let dx = l / (num_spatial - 1) as f64;

        let dt_hours = time_hours / (n - 1) as f64;
        let dt_seconds = dt_hours * 3600.0;

        let max_d = self.config.diffusion_coefficient.max(1e-20);
        let stability_limit = 0.4 * dx * dx / max_d;
        let sub_steps = ((dt_seconds / stability_limit).ceil() as usize).max(1);
        let sub_dt = dt_seconds / sub_steps as f64;

        let mut concentration = vec![c0; num_spatial];

        let mut time_points = Vec::with_capacity(n);
        let mut moisture_values = Vec::with_capacity(n);

        time_points.push(0.0);
        moisture_values.push(c0);

        for step in 1..n {
            for _sub in 0..sub_steps {
                let mut new_concentration = concentration.clone();

                for i in 1..(num_spatial - 1) {
                    let c_left = concentration[i - 1];
                    let c_center = concentration[i];
                    let c_right = concentration[i + 1];

                    let d_left = self.effective_diffusion_coefficient(
                        max_d,
                        (c_left + c_center) / 2.0,
                        c0,
                    );
                    let d_right = self.effective_diffusion_coefficient(
                        max_d,
                        (c_center + c_right) / 2.0,
                        c0,
                    );

                    let flux = (d_right * (c_right - c_center) - d_left * (c_center - c_left))
                        / (dx * dx);
                    new_concentration[i] = c_center + flux * sub_dt;
                }

                new_concentration[0] = ce;
                new_concentration[num_spatial - 1] = ce;

                for c in new_concentration.iter_mut() {
                    *c = c.max(ce).min(c0 + 10.0);
                }

                concentration = new_concentration;
            }

            let avg_moisture: f64 = concentration.iter().sum::<f64>() / concentration.len() as f64;

            let t_hours = step as f64 * dt_hours;
            time_points.push(t_hours);
            moisture_values.push(avg_moisture);
        }

        let d_avg = self.effective_diffusion_coefficient(
            max_d,
            (c0 + ce) / 2.0,
            c0,
        );
        let estimated_time = self.estimate_dehydration_time(c0, ce, d_avg, l);

        (time_points, moisture_values, estimated_time)
    }

    pub fn predict_with_depth_profile(
        &self,
        initial_moisture: f64,
        target_moisture: f64,
        time_hours: f64,
        num_time_points: Option<usize>,
    ) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64) {
        let n = num_time_points.unwrap_or(self.config.default_num_points).max(2);
        let l = self.config.thickness.max(1e-10);
        let c0 = initial_moisture;
        let ce = target_moisture;
        let num_spatial = 21;
        let dx = l / (num_spatial - 1) as f64;

        let dt_hours = time_hours / (n - 1) as f64;
        let dt_seconds = dt_hours * 3600.0;

        let max_d = self.config.diffusion_coefficient.max(1e-20);
        let stability_limit = 0.4 * dx * dx / max_d;
        let sub_steps = ((dt_seconds / stability_limit).ceil() as usize).max(1);
        let sub_dt = dt_seconds / sub_steps as f64;

        let mut concentration = vec![c0; num_spatial];
        let mut time_points = Vec::with_capacity(n);
        let mut moisture_values = Vec::with_capacity(n);
        let mut depth_profiles = Vec::with_capacity(n);

        time_points.push(0.0);
        let avg_0: f64 = concentration.iter().sum::<f64>() / concentration.len() as f64;
        moisture_values.push(avg_0);
        depth_profiles.push(concentration.clone());

        for step in 1..n {
            for _sub in 0..sub_steps {
                let mut new_concentration = concentration.clone();

                for i in 1..(num_spatial - 1) {
                    let c_left = concentration[i - 1];
                    let c_center = concentration[i];
                    let c_right = concentration[i + 1];

                    let d_left = self.effective_diffusion_coefficient(
                        max_d,
                        (c_left + c_center) / 2.0,
                        c0,
                    );
                    let d_right = self.effective_diffusion_coefficient(
                        max_d,
                        (c_center + c_right) / 2.0,
                        c0,
                    );

                    let flux = (d_right * (c_right - c_center) - d_left * (c_center - c_left))
                        / (dx * dx);
                    new_concentration[i] = c_center + flux * sub_dt;
                }

                new_concentration[0] = ce;
                new_concentration[num_spatial - 1] = ce;

                for c in new_concentration.iter_mut() {
                    *c = c.max(ce).min(c0 + 10.0);
                }

                concentration = new_concentration;
            }

            let avg: f64 = concentration.iter().sum::<f64>() / concentration.len() as f64;
            let t_hours = step as f64 * dt_hours;
            time_points.push(t_hours);
            moisture_values.push(avg);
            depth_profiles.push(concentration.clone());
        }

        let d_avg = self.effective_diffusion_coefficient(
            max_d,
            (c0 + ce) / 2.0,
            c0,
        );
        let estimated_time = self.estimate_dehydration_time(c0, ce, d_avg, l);

        (time_points, moisture_values, depth_profiles, estimated_time)
    }

    pub fn moisture_at_depth(&self, depth: f64, time_hours: f64, initial_moisture: f64, surface_moisture: f64) -> f64 {
        let t = time_hours * 3600.0;
        let d = self.config.diffusion_coefficient;
        let l = self.config.thickness;

        let mut sum = 0.0;
        for n in 0..50 {
            let n_f = n as f64;
            let term = ((-1.0_f64).powf(n_f) / (2.0 * n_f + 1.0))
                * ((PI * (2.0 * n_f + 1.0) * depth) / (2.0 * l)).cos()
                * (-d * PI * PI * (2.0 * n_f + 1.0).powi(2) * t / (4.0 * l * l)).exp();
            sum += term;
        }

        surface_moisture + (initial_moisture - surface_moisture) * (4.0 / PI) * sum
    }

    fn estimate_dehydration_time(&self, c0: f64, ce: f64, d: f64, l: f64) -> f64 {
        let target_ratio = 0.95;
        let time_seconds = (l * l) / (PI * PI * d)
            * ((PI / 4.0) * (c0 - ce) / (target_ratio * (c0 - ce)))
                .ln()
                .abs();
        time_seconds / 3600.0
    }

    pub fn diffusion_coefficient(&self) -> f64 {
        self.config.diffusion_coefficient
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> DiffusionConfig {
        DiffusionConfig {
            diffusion_coefficient: 1e-9,
            thickness: 0.01,
            crusting_alpha: 2.0,
            crust_threshold: 30.0,
            default_num_points: 100,
        }
    }

    #[test]
    fn test_fickian_diffusion() {
        let service = MoistureDiffusionService::new(test_config());
        let (times, moistures, est_time) = service.predict_loss(80.0, 12.0, 720.0, Some(100));

        assert_eq!(times.len(), 100);
        assert_eq!(moistures.len(), 100);
        assert!(moistures[0] > moistures[moistures.len() - 1]);
        assert!(est_time > 0.0);
    }

    #[test]
    fn test_crusting_slows_early_dehydration() {
        let cfg_no_crust = DiffusionConfig { crusting_alpha: 0.0, crust_threshold: 0.0, ..test_config() };
        let cfg_with_crust = test_config();

        let s_no = MoistureDiffusionService::new(cfg_no_crust);
        let s_with = MoistureDiffusionService::new(cfg_with_crust);

        let (_, m_no, _) = s_no.predict_loss(80.0, 12.0, 720.0, Some(100));
        let (_, m_with, _) = s_with.predict_loss(80.0, 12.0, 720.0, Some(100));

        let mid = 25;
        assert!(
            m_with[mid] > m_no[mid],
            "Crusting model should retain more moisture in early phase: with={} vs without={}",
            m_with[mid], m_no[mid]
        );
    }

    #[test]
    fn test_concentration_dependent_diffusion() {
        let service = MoistureDiffusionService::new(test_config());

        let d_high = service.effective_diffusion_coefficient(1e-9, 70.0, 80.0);
        let d_low = service.effective_diffusion_coefficient(1e-9, 20.0, 80.0);

        assert!(d_high > d_low, "D should decrease as concentration drops below crust threshold");
        assert!(d_low < 1e-9, "Crusting should reduce D below D0");
    }

    #[test]
    fn test_edge_case_zero_thickness() {
        let cfg = DiffusionConfig { thickness: 0.0, ..test_config() };
        let service = MoistureDiffusionService::new(cfg);
        let (times, moistures, _) = service.predict_loss(80.0, 12.0, 720.0, Some(10));
        assert_eq!(times.len(), 10);
        assert!(moistures.iter().all(|m| m.is_finite()));
    }

    #[test]
    fn test_edge_case_single_point() {
        let service = MoistureDiffusionService::new(test_config());
        let (times, moistures, _) = service.predict_loss(80.0, 12.0, 720.0, Some(1));
        assert_eq!(times.len(), 2);
        assert_eq!(moistures.len(), 2);
    }

    #[test]
    fn test_edge_case_zero_diffusion() {
        let cfg = DiffusionConfig { diffusion_coefficient: 0.0, ..test_config() };
        let service = MoistureDiffusionService::new(cfg);
        let (times, moistures, _) = service.predict_loss(80.0, 12.0, 720.0, Some(10));
        assert_eq!(times.len(), 10);
        assert!(moistures.iter().all(|m| m.is_finite()));
    }
}
