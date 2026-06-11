use std::f64::consts::PI;

pub struct FickianDiffusionModel {
    pub diffusion_coefficient: f64,
    pub thickness: f64,
    pub crusting_alpha: f64,
    pub crust_threshold: f64,
}

impl FickianDiffusionModel {
    pub fn new(diffusion_coefficient: Option<f64>, thickness: Option<f64>) -> Self {
        Self {
            diffusion_coefficient: diffusion_coefficient.unwrap_or(1e-9),
            thickness: thickness.unwrap_or(0.01),
            crusting_alpha: 2.0,
            crust_threshold: 30.0,
        }
    }

    pub fn with_crusting(mut self, alpha: f64, threshold: f64) -> Self {
        self.crusting_alpha = alpha;
        self.crust_threshold = threshold;
        self
    }

    fn effective_diffusion_coefficient(&self, d0: f64, current_concentration: f64, initial_concentration: f64) -> f64 {
        let c_ratio = current_concentration / initial_concentration.max(1.0);
        if current_concentration < self.crust_threshold {
            let crust_factor = (current_concentration / self.crust_threshold).max(0.05);
            d0 * crust_factor.powf(self.crusting_alpha)
        } else {
            d0 * (1.0 - 0.3 * (1.0 - c_ratio))
        }
    }

    pub fn predict_moisture_loss(
        &self,
        initial_moisture: f64,
        target_moisture: f64,
        time_hours: f64,
        num_points: usize,
    ) -> (Vec<f64>, Vec<f64>, f64) {
        let num_points = num_points.max(2);
        let l = self.thickness.max(1e-10);
        let c0 = initial_moisture;
        let ce = target_moisture;
        let num_spatial = 21;
        let dx = l / (num_spatial - 1) as f64;

        let dt_hours = time_hours / (num_points - 1) as f64;
        let dt_seconds = dt_hours * 3600.0;

        let max_d = self.diffusion_coefficient.max(1e-20);
        let stability_limit = 0.4 * dx * dx / max_d;
        let sub_steps = ((dt_seconds / stability_limit).ceil() as usize).max(1);
        let sub_dt = dt_seconds / sub_steps as f64;

        let mut concentration = vec![c0; num_spatial];

        let mut time_points = Vec::with_capacity(num_points);
        let mut moisture_values = Vec::with_capacity(num_points);

        time_points.push(0.0);
        moisture_values.push(c0);

        for step in 1..num_points {
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

    pub fn calculate_mt(&self, c0: f64, ce: f64, d: f64, l: f64, t: f64) -> f64 {
        if t <= 0.0 {
            return c0;
        }

        let x = l / 2.0;
        let mut sum = 0.0;

        for n in 0..50 {
            let n_f = n as f64;
            let term = ((-1.0_f64).powf(n_f) / (2.0 * n_f + 1.0))
                * ((PI * (2.0 * n_f + 1.0) * x) / (2.0 * l)).cos()
                * (-d * PI * PI * (2.0 * n_f + 1.0).powi(2) * t / (4.0 * l * l)).exp();
            sum += term;
        }

        let mt = ce + (c0 - ce) * (4.0 / PI) * sum;
        mt.max(ce)
    }

    fn estimate_dehydration_time(&self, c0: f64, ce: f64, d: f64, l: f64) -> f64 {
        let target_ratio = 0.95;
        let time_seconds = (l * l) / (PI * PI * d)
            * ((PI / 4.0) * (c0 - ce) / (target_ratio * (c0 - ce)))
                .ln()
                .abs();
        time_seconds / 3600.0
    }

    pub fn moisture_at_depth(&self, depth: f64, time_hours: f64, initial_moisture: f64, surface_moisture: f64) -> f64 {
        let t = time_hours * 3600.0;
        let d = self.diffusion_coefficient;
        let l = self.thickness;

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

    pub fn predict_with_depth_profile(
        &self,
        initial_moisture: f64,
        target_moisture: f64,
        time_hours: f64,
        num_time_points: usize,
    ) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64) {
        let num_time_points = num_time_points.max(2);
        let l = self.thickness.max(1e-10);
        let c0 = initial_moisture;
        let ce = target_moisture;
        let num_spatial = 21;
        let dx = l / (num_spatial - 1) as f64;

        let dt_hours = time_hours / (num_time_points - 1) as f64;
        let dt_seconds = dt_hours * 3600.0;

        let max_d = self.diffusion_coefficient.max(1e-20);
        let stability_limit = 0.4 * dx * dx / max_d;
        let sub_steps = ((dt_seconds / stability_limit).ceil() as usize).max(1);
        let sub_dt = dt_seconds / sub_steps as f64;

        let mut concentration = vec![c0; num_spatial];
        let mut time_points = Vec::with_capacity(num_time_points);
        let mut moisture_values = Vec::with_capacity(num_time_points);
        let mut depth_profiles = Vec::with_capacity(num_time_points);

        time_points.push(0.0);
        let avg_0: f64 = concentration.iter().sum::<f64>() / concentration.len() as f64;
        moisture_values.push(avg_0);
        depth_profiles.push(concentration.clone());

        for step in 1..num_time_points {
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
}

pub struct DarcyLawModel {
    pub permeability: f64,
    pub viscosity: f64,
    pub porosity: f64,
}

impl DarcyLawModel {
    pub fn new(viscosity: f64, permeability: Option<f64>) -> Self {
        Self {
            permeability: permeability.unwrap_or(1e-14),
            viscosity,
            porosity: 0.4,
        }
    }

    pub fn predict_penetration(
        &self,
        time_hours: f64,
        pressure_diff: f64,
        sample_length: f64,
        num_points: usize,
    ) -> (Vec<f64>, Vec<f64>, f64) {
        let mut time_points = Vec::with_capacity(num_points);
        let mut depth_values = Vec::with_capacity(num_points);

        let k = self.permeability;
        let mu = self.viscosity;
        let phi = self.porosity;
        let delta_p = pressure_diff;

        for i in 0..num_points {
            let t_hours = (time_hours * i as f64) / (num_points - 1).max(1) as f64;
            let t_seconds = t_hours * 3600.0;

            let depth = ((2.0 * k * delta_p * t_seconds) / (mu * phi)).sqrt();
            let depth_mm = depth * 1000.0;

            time_points.push(t_hours);
            depth_values.push(depth_mm.min(sample_length * 1000.0));
        }

        let final_depth = ((2.0 * k * delta_p * time_hours * 3600.0) / (mu * phi)).sqrt() * 1000.0;

        (time_points, depth_values, final_depth.min(sample_length * 1000.0))
    }

    pub fn flow_rate(&self, pressure_diff: f64, area: f64, length: f64) -> f64 {
        let k = self.permeability;
        let mu = self.viscosity;
        (k * area * pressure_diff) / (mu * length)
    }

    pub fn penetration_velocity(&self, pressure_diff: f64, current_depth: f64) -> f64 {
        if current_depth <= 0.0 {
            return f64::INFINITY;
        }
        let k = self.permeability;
        let mu = self.viscosity;
        let phi = self.porosity;
        (k * pressure_diff) / (mu * phi * current_depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fickian_diffusion() {
        let model = FickianDiffusionModel::new(Some(1e-9), Some(0.01));
        let (times, moistures, est_time) = model.predict_moisture_loss(80.0, 12.0, 720.0, 100);

        assert_eq!(times.len(), 100);
        assert_eq!(moistures.len(), 100);
        assert!(moistures[0] > moistures[moistures.len() - 1]);
        assert!(est_time > 0.0);
    }

    #[test]
    fn test_crusting_slows_early_dehydration() {
        let model_no_crust = FickianDiffusionModel::new(Some(1e-9), Some(0.01))
            .with_crusting(0.0, 0.0);
        let model_with_crust = FickianDiffusionModel::new(Some(1e-9), Some(0.01))
            .with_crusting(2.0, 30.0);

        let (_, m_no_crust, _) = model_no_crust.predict_moisture_loss(80.0, 12.0, 720.0, 100);
        let (_, m_with_crust, _) = model_with_crust.predict_moisture_loss(80.0, 12.0, 720.0, 100);

        let mid = 25;
        assert!(
            m_with_crust[mid] > m_no_crust[mid],
            "Crusting model should retain more moisture in early phase: with={} vs without={}",
            m_with_crust[mid],
            m_no_crust[mid]
        );
    }

    #[test]
    fn test_concentration_dependent_diffusion() {
        let model = FickianDiffusionModel::new(Some(1e-9), Some(0.01));

        let d_high = model.effective_diffusion_coefficient(1e-9, 70.0, 80.0);
        let d_low = model.effective_diffusion_coefficient(1e-9, 20.0, 80.0);

        assert!(d_high > d_low, "D should decrease as concentration drops below crust threshold");
        assert!(d_low < 1e-9, "Crusting should reduce D below D0");
    }

    #[test]
    fn test_darcy_law() {
        let model = DarcyLawModel::new(0.056, Some(1e-14));
        let (times, depths, final_depth) = model.predict_penetration(48.0, 101325.0, 0.05, 50);

        assert_eq!(times.len(), 50);
        assert_eq!(depths.len(), 50);
        assert!(depths[0] < depths[depths.len() - 1]);
        assert!(final_depth > 0.0);
    }

    #[test]
    fn test_edge_case_zero_thickness() {
        let model = FickianDiffusionModel::new(Some(1e-9), Some(0.0));
        let (times, moistures, _) = model.predict_moisture_loss(80.0, 12.0, 720.0, 10);
        assert_eq!(times.len(), 10);
        assert!(moistures.iter().all(|m| m.is_finite()));
    }

    #[test]
    fn test_edge_case_single_point() {
        let model = FickianDiffusionModel::new(Some(1e-9), Some(0.01));
        let (times, moistures, _) = model.predict_moisture_loss(80.0, 12.0, 720.0, 1);
        assert_eq!(times.len(), 2);
        assert_eq!(moistures.len(), 2);
    }

    #[test]
    fn test_edge_case_zero_diffusion() {
        let model = FickianDiffusionModel::new(Some(0.0), Some(0.01));
        let (times, moistures, _) = model.predict_moisture_loss(80.0, 12.0, 720.0, 10);
        assert_eq!(times.len(), 10);
        assert!(moistures.iter().all(|m| m.is_finite()));
    }
}
