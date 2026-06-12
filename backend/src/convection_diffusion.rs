use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvectionDiffusionConfig {
    pub diffusion_coefficient: f64,
    pub permeability: f64,
    pub viscosity: f64,
    pub pressure_gradient: f64,
    pub porosity: f64,
    pub tortuosity: f64,
    pub initial_concentration: f64,
    pub surface_concentration: f64,
    pub molecular_weight: f64,
    pub num_grid_x: usize,
    pub num_grid_y: usize,
    pub thickness: f64,
    pub width: f64,
    pub total_time_hours: f64,
    pub time_steps: usize,
}

impl Default for ConvectionDiffusionConfig {
    fn default() -> Self {
        Self {
            diffusion_coefficient: 1e-10,
            permeability: 1e-15,
            viscosity: 0.056,
            pressure_gradient: 101325.0,
            porosity: 0.4,
            tortuosity: 2.5,
            initial_concentration: 0.0,
            surface_concentration: 30.0,
            molecular_weight: 300.0,
            num_grid_x: 40,
            num_grid_y: 30,
            thickness: 0.05,
            width: 0.2,
            total_time_hours: 168.0,
            time_steps: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConcentrationFieldResult {
    pub grid_x: Vec<f64>,
    pub grid_y: Vec<f64>,
    pub concentration: Vec<Vec<f64>>,
    pub time_points: Vec<f64>,
    pub concentration_at_times: Vec<Vec<Vec<f64>>>,
    pub penetration_front_x: Vec<f64>,
    pub penetration_front_y: Vec<f64>,
    pub penetration_depth_time: Vec<f64>,
    pub penetration_depth_values: Vec<f64>,
    pub darcy_velocity: f64,
    pub effective_diffusion: f64,
    pub peclet_number: f64,
    pub avg_concentration: f64,
    pub max_concentration: f64,
    pub front_velocity: f64,
    pub concentration_profile_centerline: Vec<f64>,
}

#[derive(Clone)]
pub struct ConvectionDiffusionSolver {
    config: ConvectionDiffusionConfig,
}

impl ConvectionDiffusionSolver {
    pub fn new(config: ConvectionDiffusionConfig) -> Self {
        Self { config }
    }

    pub fn darcy_velocity(&self) -> f64 {
        (self.config.permeability * self.config.pressure_gradient)
            / (self.config.viscosity * self.config.thickness)
    }

    pub fn effective_diffusion(&self) -> f64 {
        self.config.diffusion_coefficient * self.config.porosity / self.config.tortuosity
    }

    pub fn peclet_number(&self) -> f64 {
        let v = self.darcy_velocity();
        let d_eff = self.effective_diffusion();
        let l_char = self.config.thickness;
        (v * l_char) / d_eff
    }

    pub fn solve(&self) -> ConcentrationFieldResult {
        let nx = self.config.num_grid_x;
        let ny = self.config.num_grid_y;
        let nt = self.config.time_steps;

        let dx = self.config.width / (nx - 1) as f64;
        let dy = self.config.thickness / (ny - 1) as f64;
        let dt = (self.config.total_time_hours * 3600.0) / nt as f64;

        let v_darcy = self.darcy_velocity();
        let d_eff = self.effective_diffusion();

        let v_x = v_darcy * 0.0;
        let v_y = v_darcy;

        let d_x = d_eff;
        let d_y = d_eff;

        let r_x = d_x * dt / (dx * dx);
        let r_y = d_y * dt / (dy * dy);
        let _pe_cell_y = (v_y * dy) / d_y;

        assert!(r_x < 0.5 && r_y < 0.5, "FTCS stability condition violated");

        let mut grid_x = vec![0.0; nx];
        let mut grid_y = vec![0.0; ny];
        for i in 0..nx {
            grid_x[i] = i as f64 * dx;
        }
        for j in 0..ny {
            grid_y[j] = j as f64 * dy;
        }

        let mut concentration = vec![vec![self.config.initial_concentration; nx]; ny];

        let sample_times = vec![
            0.0,
            self.config.total_time_hours * 0.1,
            self.config.total_time_hours * 0.25,
            self.config.total_time_hours * 0.5,
            self.config.total_time_hours * 0.75,
            self.config.total_time_hours,
        ];
        let mut concentration_at_times = Vec::new();
        let mut next_sample_idx = 0;

        let mut time_points = Vec::new();
        let mut penetration_depth_values = Vec::new();

        for step in 0..nt {
            let current_time_h = (step as f64 + 1.0) * dt / 3600.0;

            let mut new_conc = concentration.clone();

            for j in 1..(ny - 1) {
                for i in 1..(nx - 1) {
                    let c = concentration[j][i];

                    let c_left = concentration[j][i - 1];
                    let c_right = concentration[j][i + 1];
                    let c_down = concentration[j - 1][i];
                    let c_up = concentration[j + 1][i];

                    let diffusion_term = d_x * (c_right - 2.0 * c + c_left) / (dx * dx)
                        + d_y * (c_up - 2.0 * c + c_down) / (dy * dy);

                    let convection_x = if v_x >= 0.0 {
                        v_x * (c - c_left) / dx
                    } else {
                        v_x * (c_right - c) / dx
                    };
                    let convection_y = if v_y >= 0.0 {
                        v_y * (c - c_down) / dy
                    } else {
                        v_y * (c_up - c) / dy
                    };

                    let dc_dt = diffusion_term - convection_x - convection_y;
                    new_conc[j][i] = c + dc_dt * dt;
                    new_conc[j][i] = new_conc[j][i]
                        .max(self.config.initial_concentration)
                        .min(self.config.surface_concentration * 1.1);
                }
            }

            for i in 0..nx {
                new_conc[0][i] = self.config.surface_concentration;
                new_conc[ny - 1][i] = self.calculate_bottom_bc(concentration[ny - 1][i], concentration[ny - 2][i], dy, d_y, v_y);
            }

            for j in 0..ny {
                new_conc[j][0] = new_conc[j][1];
                new_conc[j][nx - 1] = new_conc[j][nx - 2];
            }

            concentration = new_conc;

            if next_sample_idx < sample_times.len() && current_time_h >= sample_times[next_sample_idx] {
                concentration_at_times.push(concentration.clone());
                next_sample_idx += 1;
            }

            if step % (nt / 20).max(1) == 0 || step == nt - 1 {
                time_points.push(current_time_h);
                let front_depth = self.calculate_penetration_depth(&concentration, dy);
                penetration_depth_values.push(front_depth);
            }
        }

        let (front_x, front_y) = self.calculate_penetration_front(&concentration, &grid_x, &grid_y);

        let profile_centerline = (0..ny).map(|j| concentration[j][nx / 2]).collect();

        let mut avg_c = 0.0;
        let mut max_c = 0.0;
        for j in 0..ny {
            for i in 0..nx {
                avg_c += concentration[j][i];
                if concentration[j][i] > max_c {
                    max_c = concentration[j][i];
                }
            }
        }
        avg_c /= (nx * ny) as f64;

        let front_velocity = if penetration_depth_values.len() >= 2 {
            let last_idx = penetration_depth_values.len() - 1;
            (penetration_depth_values[last_idx] - penetration_depth_values[0])
                / (time_points[last_idx] - time_points[0] + 1e-6).max(1e-6)
        } else {
            0.0
        };

        ConcentrationFieldResult {
            grid_x,
            grid_y,
            concentration,
            time_points: time_points.clone(),
            concentration_at_times,
            penetration_front_x: front_x,
            penetration_front_y: front_y,
            penetration_depth_time: time_points,
            penetration_depth_values,
            darcy_velocity: v_darcy,
            effective_diffusion: d_eff,
            peclet_number: self.peclet_number(),
            avg_concentration: avg_c,
            max_concentration: max_c,
            front_velocity,
            concentration_profile_centerline: profile_centerline,
        }
    }

    fn calculate_bottom_bc(&self, _c_bottom: f64, c_above: f64, _dy: f64, _d: f64, _v: f64) -> f64 {
        c_above
    }

    fn calculate_penetration_depth(&self, concentration: &[Vec<f64>], dy: f64) -> f64 {
        let nx = concentration[0].len();
        let ny = concentration.len();
        let threshold = self.config.surface_concentration * 0.1;

        let mut total_depth = 0.0;
        let mut count = 0;

        for i in 0..nx {
            for j in (0..ny).rev() {
                if concentration[j][i] >= threshold {
                    total_depth += j as f64 * dy;
                    count += 1;
                    break;
                }
            }
        }

        if count > 0 {
            total_depth / count as f64
        } else {
            0.0
        }
    }

    fn calculate_penetration_front(
        &self,
        concentration: &[Vec<f64>],
        grid_x: &[f64],
        _grid_y: &[f64],
    ) -> (Vec<f64>, Vec<f64>) {
        let nx = concentration[0].len();
        let ny = concentration.len();
        let threshold = self.config.surface_concentration * 0.5;
        let dy = self.config.thickness / (ny - 1) as f64;

        let mut front_x = Vec::new();
        let mut front_y = Vec::new();

        for i in 0..nx {
            let mut front_j = 0;
            for j in 0..ny {
                if concentration[j][i] < threshold {
                    front_j = j;
                    break;
                }
            }
            if front_j > 0 {
                let y_interp = if front_j > 0 {
                    let c_above = concentration[front_j - 1][i];
                    let c_below = concentration[front_j][i];
                    if (c_above - c_below).abs() > 1e-6 {
                        (front_j - 1) as f64 + (c_above - threshold) / (c_above - c_below)
                    } else {
                        (front_j - 1) as f64
                    }
                } else {
                    0.0
                };

                front_x.push(grid_x[i]);
                front_y.push(y_interp * dy);
            }
        }

        (front_x, front_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_darcy_velocity() {
        let config = ConvectionDiffusionConfig::default();
        let solver = ConvectionDiffusionSolver::new(config);

        let v = solver.darcy_velocity();
        assert!(v > 0.0, "Darcy velocity should be positive");
        assert!(v < 1e-3, "Darcy velocity should be very small for wood");
    }

    #[test]
    fn test_effective_diffusion() {
        let config = ConvectionDiffusionConfig::default();
        let bulk_d = config.diffusion_coefficient;
        let solver = ConvectionDiffusionSolver::new(config);

        let d_eff = solver.effective_diffusion();
        assert!(d_eff > 0.0);
        assert!(d_eff < bulk_d,
                "Effective diffusion should be less than bulk diffusion due to tortuosity");
    }

    #[test]
    fn test_concentration_basic() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 20,
            num_grid_y: 15,
            total_time_hours: 10.0,
            time_steps: 50,
            ..Default::default()
        };
        let surface_conc = config.surface_concentration;
        let solver = ConvectionDiffusionSolver::new(config);

        let result = solver.solve();
        assert_eq!(result.concentration.len(), 15);
        assert_eq!(result.concentration[0].len(), 20);
        assert!(result.avg_concentration >= 0.0);
        assert!(result.max_concentration <= surface_conc * 1.1);
    }

    #[test]
    fn test_penetration_increases_with_time() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 15,
            num_grid_y: 20,
            total_time_hours: 48.0,
            time_steps: 100,
            ..Default::default()
        };
        let solver = ConvectionDiffusionSolver::new(config);

        let result = solver.solve();
        assert!(result.penetration_depth_values.len() >= 2);

        let first = result.penetration_depth_values[0];
        let last = result.penetration_depth_values[result.penetration_depth_values.len() - 1];
        assert!(last >= first, "Penetration depth should increase with time");
    }

    #[test]
    fn test_peclet_number() {
        let config = ConvectionDiffusionConfig::default();
        let solver = ConvectionDiffusionSolver::new(config);

        let pe = solver.peclet_number();
        assert!(pe.is_finite());
        assert!(pe > 0.0);
    }

    #[test]
    fn test_peg_front_vs_mri_measurement() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 30,
            num_grid_y: 60,
            total_time_hours: 160.0,
            time_steps: 700,
            thickness: 0.05,
            surface_concentration: 0.4,
            diffusion_coefficient: 1.8e-10,
            porosity: 0.45,
            tortuosity: 2.5,
            permeability: 1.5e-16,
            pressure_gradient: 120000.0,
            ..Default::default()
        };
        let _dy = config.thickness / (config.num_grid_y - 1) as f64;
        let solver = ConvectionDiffusionSolver::new(config.clone());
        let result = solver.solve();

        let mri_measured_depth_mm = 12.5;
        let predicted_depth_mm = result.penetration_depth_values.last().copied().unwrap_or(0.0) * 1000.0;
        let error_mm = (predicted_depth_mm - mri_measured_depth_mm).abs();

        assert!(error_mm < 2.0,
                "PEG front prediction error ({:.2} mm) should be < 2 mm (predicted={:.2} mm, MRI={:.2} mm)",
                error_mm, predicted_depth_mm, mri_measured_depth_mm);
        assert!(result.penetration_depth_time.len() >= 2);
    }

    #[test]
    fn test_analytical_vs_numerical_penetration() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 20,
            num_grid_y: 50,
            total_time_hours: 24.0,
            time_steps: 500,
            thickness: 0.02,
            diffusion_coefficient: 5e-11,
            permeability: 1e-18,
            ..Default::default()
        };
        let solver = ConvectionDiffusionSolver::new(config.clone());
        let result = solver.solve();

        let d_eff = solver.effective_diffusion();
        let t_sec = config.total_time_hours * 3600.0;
        let analytical_depth_m = 2.0 * (d_eff * t_sec).sqrt();
        let analytical_depth_mm = analytical_depth_m * 1000.0;
        let numerical_depth_mm = result.penetration_depth_values.last().copied().unwrap_or(0.0) * 1000.0;

        let error_mm = (analytical_depth_mm - numerical_depth_mm).abs();
        assert!(error_mm < 2.0,
                "Numerical vs analytical penetration error ({:.2} mm) < 2 mm (analytical={:.2}, numerical={:.2})",
                error_mm, analytical_depth_mm, numerical_depth_mm);
    }

    #[test]
    fn test_concentration_conservation_boundary() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 10,
            num_grid_y: 20,
            total_time_hours: 1.0,
            time_steps: 10,
            surface_concentration: 0.5,
            ..Default::default()
        };
        let _dy = config.thickness / (config.num_grid_y - 1) as f64;
        let solver = ConvectionDiffusionSolver::new(config.clone());
        let result = solver.solve();

        for row in &result.concentration {
            for &c in row {
                assert!(c >= -1e-9 && c <= config.surface_concentration * 1.01,
                        "Concentration {:.4} out of bounds [0, {:.2}]", c, config.surface_concentration);
            }
        }

        for i in 0..config.num_grid_x {
            assert!((result.concentration[0][i] - config.surface_concentration).abs() < 0.01,
                    "Surface concentration should be maintained");
        }
    }

    #[test]
    fn test_zero_diffusion_anomaly() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 10,
            num_grid_y: 10,
            total_time_hours: 100.0,
            time_steps: 50,
            diffusion_coefficient: 0.0,
            permeability: 0.0,
            surface_concentration: 0.5,
            ..Default::default()
        };
        let solver = ConvectionDiffusionSolver::new(config.clone());
        let result = solver.solve();

        for j in 1..config.num_grid_y {
            for i in 0..config.num_grid_x {
                assert!(result.concentration[j][i].abs() < 1e-9,
                        "With zero transport, interior should remain zero");
            }
        }
        let final_depth = result.penetration_depth_values.last().copied().unwrap_or(0.0);
        assert!(final_depth < 0.001,
                "Penetration depth should be near zero with no transport");
    }

    #[test]
    fn test_single_grid_boundary() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 2,
            num_grid_y: 2,
            total_time_hours: 1.0,
            time_steps: 1,
            ..Default::default()
        };
        let solver = ConvectionDiffusionSolver::new(config);
        let result = solver.solve();

        assert_eq!(result.concentration.len(), 2);
        assert_eq!(result.concentration[0].len(), 2);
        let final_depth = result.penetration_depth_values.last().copied().unwrap_or(0.0);
        assert!(final_depth >= 0.0);
    }

    #[test]
    fn test_high_peclet_regime() {
        let config = ConvectionDiffusionConfig {
            num_grid_x: 15,
            num_grid_y: 25,
            total_time_hours: 48.0,
            time_steps: 200,
            permeability: 1e-12,
            viscosity: 0.001,
            diffusion_coefficient: 1e-12,
            ..Default::default()
        };
        let solver = ConvectionDiffusionSolver::new(config.clone());

        let pe = solver.peclet_number();
        assert!(pe > 10.0, "High Pe regime should have Pe > 10 (got {:.1})", pe);

        let result = solver.solve();
        let final_depth = result.penetration_depth_values.last().copied().unwrap_or(0.0);
        assert!(final_depth > 0.001,
                "Advection-dominated flow should penetrate significantly");
    }
}
