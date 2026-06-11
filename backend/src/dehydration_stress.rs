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
        let nx = self.config.num_elements_x + 1;
        let ny = self.config.num_elements_y + 1;
        let total_nodes = nx * ny;

        let mut node_x = vec![0.0; total_nodes];
        let mut node_y = vec![0.0; total_nodes];
        let mut sigma_x = vec![0.0; total_nodes];
        let mut sigma_y = vec![0.0; total_nodes];
        let mut sigma_von_mises = vec![0.0; total_nodes];
        let mut max_principal = vec![0.0; total_nodes];
        let mut min_principal = vec![0.0; total_nodes];
        let mut stress_gradient_x = vec![0.0; total_nodes];
        let mut stress_gradient_y = vec![0.0; total_nodes];

        let dx = self.config.width / (nx - 1) as f64;
        let dy = self.config.height / (ny - 1) as f64;

        for j in 0..ny {
            for i in 0..nx {
                let idx = j * nx + i;
                node_x[idx] = i as f64 * dx;
                node_y[idx] = j as f64 * dy;

                let m = moisture_profile[j][i];
                let m_ref = moisture_profile[ny / 2][nx / 2];
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
                stress_gradient_x[idx] = (sigma_von_mises[j * nx + i + 1] - sigma_von_mises[j * nx + i - 1]) / (2.0 * dx);
                stress_gradient_y[idx] = (sigma_von_mises[(j + 1) * nx + i] - sigma_von_mises[(j - 1) * nx + i]) / (2.0 * dy);
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

        let danger_zones = self.identify_danger_zones(&sigma_von_mises, &node_x, &node_y, nx, ny);

        let safety_factor = if max_vm > 0.0 {
            self.config.tensile_strength / max_vm
        } else {
            999.0
        };

        StressResult {
            node_x,
            node_y,
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

    fn identify_danger_zones(
        &self,
        sigma_von_mises: &[f64],
        _node_x: &[f64],
        _node_y: &[f64],
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
                        i, j, nx, ny, sigma_von_mises, &mut visited, threshold,
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
    ) -> (f64, f64, f64, usize) {
        let mut stack = vec![(start_i, start_j)];
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut max_val = 0.0;
        let mut count = 0;

        let dx = self.config.width / (nx - 1) as f64;
        let dy = self.config.height / (ny - 1) as f64;

        while let Some((i, j)) = stack.pop() {
            let idx = j * nx + i;
            if visited[idx] || values[idx] <= threshold {
                continue;
            }

            visited[idx] = true;
            let x = i as f64 * dx;
            let y = j as f64 * dy;
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
        let config = StressConfig::default();
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
}
