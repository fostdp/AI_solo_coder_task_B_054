use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfig {
    pub young_modulus: f64,
    pub shrinkage_coefficient: f64,
    pub poisson_ratio: f64,
    pub tensile_strength: f64,
    pub num_elements_x: usize,
    pub num_elements_y: usize,
    pub thickness: f64,
    pub width: f64,
    pub height: f64,
    pub enable_adaptive_mesh: bool,
    pub gradient_refinement_threshold: f64,
    pub max_refinement_levels: usize,
}

impl Default for StressConfig {
    fn default() -> Self {
        Self {
            young_modulus: 10.0e9,
            shrinkage_coefficient: 0.003,
            poisson_ratio: 0.35,
            tensile_strength: 40.0e6,
            num_elements_x: 20,
            num_elements_y: 20,
            thickness: 0.05,
            width: 0.2,
            height: 0.15,
            enable_adaptive_mesh: true,
            gradient_refinement_threshold: 5.0,
            max_refinement_levels: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StressResult {
    pub node_x: Vec<f64>,
    pub node_y: Vec<f64>,
    pub sigma_x: Vec<f64>,
    pub sigma_y: Vec<f64>,
    pub sigma_von_mises: Vec<f64>,
    pub max_principal: Vec<f64>,
    pub min_principal: Vec<f64>,
    pub stress_gradient_x: Vec<f64>,
    pub stress_gradient_y: Vec<f64>,
    pub danger_zones: Vec<DangerZone>,
    pub max_von_mises: f64,
    pub safety_factor: f64,
    pub avg_sigma_x: f64,
    pub avg_sigma_y: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DangerZone {
    pub center_x: f64,
    pub center_y: f64,
    pub area_percent: f64,
    pub max_stress: f64,
    pub safety_factor: f64,
    pub risk_level: String,
}

#[derive(Clone)]
pub struct DehydrationStressSolver {
    config: StressConfig,
}

impl DehydrationStressSolver {
    pub fn new(config: StressConfig) -> Self {
        Self { config }
    }

    pub fn compute_stress_field(&self, moisture_profile: &[Vec<f64>]) -> StressResult {
        if moisture_profile.is_empty() || moisture_profile[0].is_empty() {
            return StressResult {
                node_x: Vec::new(),
                node_y: Vec::new(),
                sigma_x: Vec::new(),
                sigma_y: Vec::new(),
                sigma_von_mises: Vec::new(),
                max_principal: Vec::new(),
                min_principal: Vec::new(),
                stress_gradient_x: Vec::new(),
                stress_gradient_y: Vec::new(),
                danger_zones: Vec::new(),
                max_von_mises: 0.0,
                safety_factor: 0.0,
                avg_sigma_x: 0.0,
                avg_sigma_y: 0.0,
            };
        }

        let (refined_x, refined_y, refined_moisture) = if self.config.enable_adaptive_mesh {
            self.adaptive_refine(moisture_profile)
        } else {
            let nx = self.config.num_elements_x + 1;
            let ny = self.config.num_elements_y + 1;
            let dx = self.config.width / (nx - 1) as f64;
            let dy = self.config.height / (ny - 1) as f64;
            let node_x: Vec<f64> = (0..nx).map(|i| i as f64 * dx).collect();
            let node_y: Vec<f64> = (0..ny).map(|j| j as f64 * dy).collect();
            (node_x, node_y, moisture_profile.to_vec())
        };

        let nx = refined_x.len();
        let ny = refined_y.len();
        let total_nodes = nx * ny;

        let mut node_x_out = vec![0.0; total_nodes];
        let mut node_y_out = vec![0.0; total_nodes];
        let mut sigma_x = vec![0.0; total_nodes];
        let mut sigma_y = vec![0.0; total_nodes];
        let mut sigma_von_mises = vec![0.0; total_nodes];
        let mut max_principal = vec![0.0; total_nodes];
        let mut min_principal = vec![0.0; total_nodes];
        let mut stress_gradient_x = vec![0.0; total_nodes];
        let mut stress_gradient_y = vec![0.0; total_nodes];

        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                node_x_out[idx] = refined_x[i];
                node_y_out[idx] = refined_y[j];

                let m = refined_moisture[j][i];
                let m_ref = refined_moisture[ny / 2][nx / 2];
                let delta_m = m - m_ref;

                let shrinkage_strain_x = -self.config.shrinkage_coefficient * delta_m;
                let shrinkage_strain_y = -self.config.shrinkage_coefficient * delta_m;

                let e = self.config.young_modulus;
                let nu = self.config.poisson_ratio;
                let factor = e / (1.0 - nu * nu);

                sigma_x[idx] = factor * (shrinkage_strain_x + nu * shrinkage_strain_y);
                sigma_y[idx] = factor * (shrinkage_strain_y + nu * shrinkage_strain_x);

                let sig_x = sigma_x[idx];
                let sig_y = sigma_y[idx];
                let tau_xy = 0.0;

                sigma_von_mises[idx] = (sig_x * sig_x - sig_x * sig_y + sig_y * sig_y + 3.0 * tau_xy * tau_xy).sqrt();

                let avg = (sig_x + sig_y) / 2.0;
                let radius = ((sig_x - sig_y) / 2.0).powi(2) + tau_xy * tau_xy;
                max_principal[idx] = avg + radius.sqrt();
                min_principal[idx] = avg - radius.sqrt();
            }
        }

        for j in 1..(ny - 1) {
            for i in 1..(nx - 1) {
                let idx = j * nx + i;
                let dx_local = refined_x[i + 1] - refined_x[i - 1];
                let dy_local = refined_y[j + 1] - refined_y[j - 1];
                if dx_local.abs() > 1e-15 {
                    stress_gradient_x[idx] = (sigma_von_mises[j * nx + i + 1] - sigma_von_mises[j * nx + i - 1]) / dx_local;
                }
                if dy_local.abs() > 1e-15 {
                    stress_gradient_y[idx] = (sigma_von_mises[(j + 1) * nx + i] - sigma_von_mises[(j - 1) * nx + i]) / dy_local;
                }
            }
        }

        let mut max_vm = 0.0;
        for &s in &sigma_von_mises {
            if s.abs() > max_vm {
                max_vm = s.abs();
            }
        }

        let avg_sx: f64 = sigma_x.iter().sum::<f64>() / total_nodes as f64;
        let avg_sy: f64 = sigma_y.iter().sum::<f64>() / total_nodes as f64;

        let danger_zones = self.identify_danger_zones(&sigma_von_mises, &node_x_out, &node_y_out, nx, ny);

        let safety_factor = if max_vm > 0.0 {
            self.config.tensile_strength / max_vm
        } else {
            999.0
        };

        StressResult {
            node_x: node_x_out,
            node_y: node_y_out,
            sigma_x,
            sigma_y,
            sigma_von_mises,
            max_principal,
            min_principal,
            stress_gradient_x,
            stress_gradient_y,
            danger_zones,
            max_von_mises: max_vm,
            safety_factor,
            avg_sigma_x: avg_sx,
            avg_sigma_y: avg_sy,
        }
    }

    fn adaptive_refine(&self, moisture_profile: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
        let orig_nx = moisture_profile[0].len();
        let orig_ny = moisture_profile.len();
        let dx = self.config.width / (orig_nx - 1) as f64;
        let dy = self.config.height / (orig_ny - 1) as f64;

        let mut current_x: Vec<f64> = (0..orig_nx).map(|i| i as f64 * dx).collect();
        let mut current_y: Vec<f64> = (0..orig_ny).map(|j| j as f64 * dy).collect();
        let mut current_moisture = moisture_profile.to_vec();

        for _level in 0..self.config.max_refinement_levels {
            let nx = current_x.len();
            let ny = current_y.len();

            let mut needs_refine_x = vec![false; nx - 1];
            let mut needs_refine_y = vec![false; ny - 1];

            for j in 1..(ny - 1) {
                for i in 1..(nx - 1) {
                    let dx_local = current_x.get(i + 1).copied().unwrap_or(current_x[nx - 1])
                        - current_x.get(i.wrapping_sub(1)).copied().unwrap_or(current_x[0]);
                    let dy_local = current_y.get(j + 1).copied().unwrap_or(current_y[ny - 1])
                        - current_y.get(j.wrapping_sub(1)).copied().unwrap_or(current_y[0]);
                    let dmdx = if dx_local.abs() > 1e-15 {
                        (current_moisture[j][i + 1] - current_moisture[j][i - 1]) / dx_local
                    } else {
                        0.0
                    };
                    let dmdy = if dy_local.abs() > 1e-15 {
                        (current_moisture[j + 1][i] - current_moisture[j - 1][i]) / dy_local
                    } else {
                        0.0
                    };
                    let grad_mag = (dmdx * dmdx + dmdy * dmdy).sqrt();

                    if grad_mag > self.config.gradient_refinement_threshold {
                        if i > 0 && i - 1 < needs_refine_x.len() {
                            needs_refine_x[i - 1] = true;
                        }
                        if i < nx - 1 && i < needs_refine_x.len() {
                            needs_refine_x[i] = true;
                        }
                        if j > 0 && j - 1 < needs_refine_y.len() {
                            needs_refine_y[j - 1] = true;
                        }
                        if j < ny - 1 && j < needs_refine_y.len() {
                            needs_refine_y[j] = true;
                        }
                    }
                }
            }

            let any_refine = needs_refine_x.iter().any(|&x| x) || needs_refine_y.iter().any(|&y| y);
            if !any_refine {
                break;
            }

            let mut new_x = Vec::new();
            for i in 0..(nx - 1) {
                new_x.push(current_x[i]);
                if needs_refine_x[i] {
                    new_x.push((current_x[i] + current_x[i + 1]) / 2.0);
                }
            }
            new_x.push(current_x[nx - 1]);

            let mut new_y = Vec::new();
            for j in 0..(ny - 1) {
                new_y.push(current_y[j]);
                if needs_refine_y[j] {
                    new_y.push((current_y[j] + current_y[j + 1]) / 2.0);
                }
            }
            new_y.push(current_y[ny - 1]);

            let new_moisture = Self::bilinear_interp(&current_moisture, &current_x, &current_y, &new_x, &new_y);

            current_x = new_x;
            current_y = new_y;
            current_moisture = new_moisture;
        }

        (current_x, current_y, current_moisture)
    }

    fn bilinear_interp(
        data: &[Vec<f64>],
        old_x: &[f64],
        old_y: &[f64],
        new_x: &[f64],
        new_y: &[f64],
    ) -> Vec<Vec<f64>> {
        let new_nx = new_x.len();
        let new_ny = new_y.len();
        let old_nx = old_x.len();
        let old_ny = old_y.len();

        let mut result = vec![vec![0.0; new_nx]; new_ny];

        for j in 0..new_ny {
            for i in 0..new_nx {
                let x = new_x[i];
                let y = new_y[j];

                let mut ii = 0;
                while ii < old_nx - 2 && old_x[ii + 1] < x + 1e-15 {
                    ii += 1;
                }
                let mut jj = 0;
                while jj < old_ny - 2 && old_y[jj + 1] < y + 1e-15 {
                    jj += 1;
                }

                let x0 = old_x[ii];
                let x1 = old_x[ii + 1];
                let y0 = old_y[jj];
                let y1 = old_y[jj + 1];

                let tx = if (x1 - x0).abs() > 1e-15 {
                    ((x - x0) / (x1 - x0)).max(0.0).min(1.0)
                } else {
                    0.0
                };
                let ty = if (y1 - y0).abs() > 1e-15 {
                    ((y - y0) / (y1 - y0)).max(0.0).min(1.0)
                } else {
                    0.0
                };

                result[j][i] = data[jj][ii] * (1.0 - tx) * (1.0 - ty)
                    + data[jj][ii + 1] * tx * (1.0 - ty)
                    + data[jj + 1][ii] * (1.0 - tx) * ty
                    + data[jj + 1][ii + 1] * tx * ty;
            }
        }

        result
    }

    fn identify_danger_zones(
        &self,
        sigma_von_mises: &[f64],
        node_x: &[f64],
        node_y: &[f64],
        nx: usize,
        ny: usize,
    ) -> Vec<DangerZone> {
        let threshold = self.config.tensile_strength * 0.7;
        let mut zones = Vec::new();
        let mut visited = vec![false; nx * ny];

        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                if !visited[idx] && sigma_von_mises[idx] > threshold {
                    let (zone_sum_x, zone_sum_y, zone_max, count) = self.flood_fill(
                        i, j, nx, ny, sigma_von_mises, &mut visited, threshold, node_x, node_y,
                    );

                    let total_nodes = (nx * ny) as f64;
                    let area_percent = (count as f64 / total_nodes) * 100.0;
                    let area_percent = area_percent.min(100.0).max(0.0);

                    let risk_level = if zone_max > self.config.tensile_strength {
                        "critical".to_string()
                    } else if zone_max > self.config.tensile_strength * 0.85 {
                        "high".to_string()
                    } else {
                        "medium".to_string()
                    };

                    zones.push(DangerZone {
                        center_x: zone_sum_x / count as f64,
                        center_y: zone_sum_y / count as f64,
                        area_percent,
                        max_stress: zone_max,
                        safety_factor: self.config.tensile_strength / zone_max,
                        risk_level,
                    });
                }
            }
        }

        zones.sort_by(|a, b| b.area_percent.partial_cmp(&a.area_percent).unwrap_or(std::cmp::Ordering::Equal));
        zones.truncate(5);
        zones
    }

    fn flood_fill(
        &self,
        start_i: usize,
        start_j: usize,
        nx: usize,
        ny: usize,
        values: &[f64],
        visited: &mut [bool],
        threshold: f64,
        node_x: &[f64],
        node_y: &[f64],
    ) -> (f64, f64, f64, usize) {
        let mut stack = vec![(start_i, start_j)];
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut max_val = 0.0;
        let mut count = 0;

        while let Some((i, j)) = stack.pop() {
            let idx = j * nx + i;
            if visited[idx] || values[idx] <= threshold {
                continue;
            }

            visited[idx] = true;
            let x = node_x[idx];
            let y = node_y[idx];
            sum_x += x;
            sum_y += y;
            if values[idx] > max_val {
                max_val = values[idx];
            }
            count += 1;

            if i > 0 {
                stack.push((i - 1, j));
            }
            if i < nx - 1 {
                stack.push((i + 1, j));
            }
            if j > 0 {
                stack.push((i, j - 1));
            }
            if j < ny - 1 {
                stack.push((i, j + 1));
            }
        }

        (sum_x, sum_y, max_val, count)
    }

    pub fn generate_moisture_profile(&self, c0: f64, ce: f64, time_hours: f64, d: f64) -> Vec<Vec<f64>> {
        let nx = self.config.num_elements_x + 1;
        let ny = self.config.num_elements_y + 1;
        let l_x = self.config.width / 2.0;
        let l_y = self.config.height / 2.0;

        let mut profile = vec![vec![0.0; nx]; ny];
        let t = time_hours * 3600.0;

        for j in 0..ny {
            for i in 0..nx {
                let x = (i as f64 / (nx - 1) as f64) * self.config.width - l_x;
                let y = (j as f64 / (ny - 1) as f64) * self.config.height - l_y;

                let mut sum_x = 0.0;
                let mut sum_y = 0.0;

                for n in 0..20 {
                    let nf = n as f64;
                    sum_x += ((-1.0_f64).powf(nf) / (2.0 * nf + 1.0))
                        * (std::f64::consts::PI * (2.0 * nf + 1.0) * x / (2.0 * l_x)).cos()
                        * (-d * std::f64::consts::PI * std::f64::consts::PI * (2.0 * nf + 1.0).powi(2) * t / (4.0 * l_x * l_x)).exp();
                }

                for n in 0..20 {
                    let nf = n as f64;
                    sum_y += ((-1.0_f64).powf(nf) / (2.0 * nf + 1.0))
                        * (std::f64::consts::PI * (2.0 * nf + 1.0) * y / (2.0 * l_y)).cos()
                        * (-d * std::f64::consts::PI * std::f64::consts::PI * (2.0 * nf + 1.0).powi(2) * t / (4.0 * l_y * l_y)).exp();
                }

                let m_x = ce + (c0 - ce) * (4.0 / std::f64::consts::PI) * sum_x;
                let m_y = ce + (c0 - ce) * (4.0 / std::f64::consts::PI) * sum_y;

                profile[j][i] = (m_x + m_y) / 2.0;
                profile[j][i] = profile[j][i].max(ce).min(c0 + 5.0);
            }
        }

        profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_basic() {
        let config = StressConfig {
            enable_adaptive_mesh: false,
            ..Default::default()
        };
        let solver = DehydrationStressSolver::new(config);

        let moisture = solver.generate_moisture_profile(80.0, 12.0, 48.0, 1e-9);
        assert_eq!(moisture.len(), 21);
        assert_eq!(moisture[0].len(), 21);

        let result = solver.compute_stress_field(&moisture);
        assert_eq!(result.sigma_x.len(), 441);
        assert!(result.max_von_mises > 0.0);
        assert!(result.safety_factor > 0.0);
    }

    #[test]
    fn test_uniform_moisture_zero_stress() {
        let config = StressConfig::default();
        let solver = DehydrationStressSolver::new(config);

        let uniform = vec![vec![50.0; 21]; 21];
        let result = solver.compute_stress_field(&uniform);

        assert!(result.max_von_mises.abs() < 1e-6, "Uniform moisture should produce near-zero stress");
    }

    #[test]
    fn test_danger_zone_identification() {
        let config = StressConfig {
            tensile_strength: 10.0e6,
            num_elements_x: 10,
            num_elements_y: 10,
            ..Default::default()
        };
        let solver = DehydrationStressSolver::new(config);

        let moisture = solver.generate_moisture_profile(80.0, 12.0, 100.0, 1e-9);
        let result = solver.compute_stress_field(&moisture);

        for zone in &result.danger_zones {
            assert!(zone.max_stress > 0.0);
            assert!(zone.area_percent >= 0.0 && zone.area_percent <= 100.0);
        }
    }

    #[test]
    fn test_stress_increases_with_gradient() {
        let config = StressConfig::default();
        let solver = DehydrationStressSolver::new(config);

        let m_short = solver.generate_moisture_profile(80.0, 12.0, 1.0, 1e-9);
        let m_long = solver.generate_moisture_profile(80.0, 12.0, 48.0, 1e-9);

        let r_short = solver.compute_stress_field(&m_short);
        let r_long = solver.compute_stress_field(&m_long);

        assert!(r_long.max_von_mises > r_short.max_von_mises * 0.5,
                "Longer dehydration should create significant stress");
    }

    #[test]
    fn test_high_gradient_region_has_max_stress() {
        let config = StressConfig {
            num_elements_x: 20,
            num_elements_y: 20,
            width: 0.1,
            height: 0.1,
            enable_adaptive_mesh: true,
            ..Default::default()
        };
        let nx = config.num_elements_x + 1;
        let ny = config.num_elements_y + 1;
        let width = config.width;
        let height = config.height;
        let solver = DehydrationStressSolver::new(config);

        let mut moisture = vec![vec![50.0; nx]; ny];

        for j in 0..ny {
            for i in 0..nx {
                let x = i as f64 / (nx - 1) as f64;
                let y = j as f64 / (ny - 1) as f64;
                let dist_from_center = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();

                if dist_from_center < 0.3 {
                    moisture[j][i] = 70.0;
                } else if dist_from_center < 0.32 {
                    let t = (dist_from_center - 0.3) / 0.02;
                    moisture[j][i] = 70.0 - t * 50.0;
                } else if dist_from_center < 0.45 {
                    moisture[j][i] = 20.0;
                } else {
                    let t = (dist_from_center - 0.45) / 0.25;
                    moisture[j][i] = 20.0 + t * 15.0;
                }
            }
        }

        moisture[ny / 2][nx / 2] = 70.0;

        let result = solver.compute_stress_field(&moisture);

        let mut max_stress = 0.0;
        let mut max_stress_idx = 0;
        for (idx, &s) in result.sigma_von_mises.iter().enumerate() {
            if s > max_stress {
                max_stress = s;
                max_stress_idx = idx;
            }
        }

        let max_x = result.node_x[max_stress_idx];
        let max_y = result.node_y[max_stress_idx];
        let max_xn = max_x / width;
        let max_yn = max_y / height;
        let dist_from_center = ((max_xn - 0.5).powi(2) + (max_yn - 0.5).powi(2)).sqrt();

        assert!(max_stress > 1e6, "Max stress should be significant (got {:.2e})", max_stress);

        let mut gradient: Vec<f64> = vec![0.0; nx * ny];
        for j in 1..(ny - 1) {
            for i in 1..(nx - 1) {
                let dm_dx = (moisture[j][i + 1] - moisture[j][i - 1]) / 2.0;
                let dm_dy = (moisture[j + 1][i] - moisture[j - 1][i]) / 2.0;
                gradient[j * nx + i] = (dm_dx * dm_dx + dm_dy * dm_dy).sqrt();
            }
        }

        let mut max_grad = 0.0;
        let mut max_grad_idx = 0;
        for (idx, &g) in gradient.iter().enumerate() {
            if g > max_grad {
                max_grad = g;
                max_grad_idx = idx;
            }
        }

        let grad_i = max_grad_idx % nx;
        let grad_j = max_grad_idx / nx;
        let grad_x = grad_i as f64 / (nx - 1) as f64;
        let grad_y = grad_j as f64 / (ny - 1) as f64;
        let grad_dist = ((grad_x - 0.5).powi(2) + (grad_y - 0.5).powi(2)).sqrt();

        assert!(grad_dist >= 0.28 && grad_dist <= 0.35,
                "Max gradient should be in transition zone (dist={:.3})", grad_dist);

        assert!(dist_from_center >= 0.15 && dist_from_center <= 0.55,
                "Max stress should occur in or near high gradient region (dist={:.3}, stress={:.2e})",
                dist_from_center, max_stress);
    }

    #[test]
    fn test_boundary_zero_moisture_gradient() {
        let config = StressConfig {
            num_elements_x: 10,
            num_elements_y: 10,
            ..Default::default()
        };
        let solver = DehydrationStressSolver::new(config);

        let mut moisture = vec![vec![50.0; 11]; 11];
        for i in 0..11 {
            moisture[0][i] = 10.0;
            moisture[10][i] = 10.0;
            moisture[i][0] = 10.0;
            moisture[i][10] = 10.0;
        }

        let result = solver.compute_stress_field(&moisture);
        assert!(result.max_von_mises > 1e6, "Boundary gradient should produce stress");
        assert!(result.safety_factor > 0.0);
    }

    #[test]
    fn test_extreme_tensile_strength_boundary() {
        let config_weak = StressConfig {
            tensile_strength: 1.0e6,
            ..Default::default()
        };
        let solver_weak = DehydrationStressSolver::new(config_weak);

        let config_strong = StressConfig {
            tensile_strength: 100.0e6,
            ..Default::default()
        };
        let solver_strong = DehydrationStressSolver::new(config_strong);

        let moisture = solver_weak.generate_moisture_profile(80.0, 12.0, 50.0, 1e-9);

        let r_weak = solver_weak.compute_stress_field(&moisture);
        let r_strong = solver_strong.compute_stress_field(&moisture);

        assert!(r_weak.safety_factor < r_strong.safety_factor,
                "Weaker material should have lower safety factor");
        assert!(r_weak.danger_zones.len() >= r_strong.danger_zones.len(),
                "Weaker material should have more danger zones");
    }

    #[test]
    fn test_empty_moisture_anomaly() {
        let config = StressConfig::default();
        let solver = DehydrationStressSolver::new(config);

        let empty: Vec<Vec<f64>> = Vec::new();
        let result = solver.compute_stress_field(&empty);
        assert_eq!(result.sigma_x.len(), 0);
        assert_eq!(result.max_von_mises, 0.0);
        assert!(result.danger_zones.is_empty());
    }

    #[test]
    fn test_single_element_boundary() {
        let config = StressConfig {
            num_elements_x: 1,
            num_elements_y: 1,
            ..Default::default()
        };
        let solver = DehydrationStressSolver::new(config);

        let moisture = vec![vec![80.0, 70.0], vec![75.0, 65.0]];
        let result = solver.compute_stress_field(&moisture);

        assert_eq!(result.sigma_x.len(), 4);
        assert!(result.max_von_mises > 0.0);
    }

    #[test]
    fn test_adaptive_mesh_convergence() {
        let config_adaptive = StressConfig {
            num_elements_x: 10,
            num_elements_y: 10,
            enable_adaptive_mesh: true,
            gradient_refinement_threshold: 10.0,
            max_refinement_levels: 2,
            ..Default::default()
        };
        let solver_adaptive = DehydrationStressSolver::new(config_adaptive);

        let config_uniform = StressConfig {
            num_elements_x: 10,
            num_elements_y: 10,
            enable_adaptive_mesh: false,
            ..Default::default()
        };
        let solver_uniform = DehydrationStressSolver::new(config_uniform);

        let moisture = solver_uniform.generate_moisture_profile(80.0, 12.0, 48.0, 1e-9);

        let result_adaptive = solver_adaptive.compute_stress_field(&moisture);
        let result_uniform = solver_uniform.compute_stress_field(&moisture);

        assert!(result_adaptive.sigma_x.len() > result_uniform.sigma_x.len(),
                "Adaptive mesh should have more nodes than uniform mesh");

        assert!(result_adaptive.max_von_mises > 0.0);
        assert!(result_adaptive.safety_factor > 0.0);

        let adaptive_node_count = result_adaptive.node_x.len();
        let uniform_node_count = result_uniform.node_x.len();
        let ratio = adaptive_node_count as f64 / uniform_node_count as f64;
        assert!(ratio >= 1.0 && ratio <= 100.0,
                "Adaptive refinement should increase node count reasonably (ratio={:.1})", ratio);
    }
}
