use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GPRConfig {
    pub length_scale: f64,
    pub signal_variance: f64,
    pub noise_variance: f64,
    pub kernel_type: String,
    pub target_moisture: f64,
    pub confidence_level: f64,
    pub max_prediction_hours: f64,
}

impl Default for GPRConfig {
    fn default() -> Self {
        Self {
            length_scale: 100.0,
            signal_variance: 25.0,
            noise_variance: 0.5,
            kernel_type: "matern52".to_string(),
            target_moisture: 15.0,
            confidence_level: 0.95,
            max_prediction_hours: 5000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GPRPredictionResult {
    pub predicted_end_time_hours: f64,
    pub confidence_lower_hours: f64,
    pub confidence_upper_hours: f64,
    pub remaining_hours: f64,
    pub confidence_interval_lower: f64,
    pub confidence_interval_upper: f64,
    pub predicted_curve_time: Vec<f64>,
    pub predicted_curve_mean: Vec<f64>,
    pub predicted_curve_lower: Vec<f64>,
    pub predicted_curve_upper: Vec<f64>,
    pub training_data_time: Vec<f64>,
    pub training_data_moisture: Vec<f64>,
    pub r_squared: f64,
    pub log_marginal_likelihood: f64,
    pub uncertainty_at_target: f64,
}

#[derive(Clone)]
pub struct GaussianProcessRegressor {
    config: GPRConfig,
    train_x: Vec<f64>,
    train_y: Vec<f64>,
    k_matrix: Vec<Vec<f64>>,
    alpha: Vec<f64>,
    l_cholesky: Vec<Vec<f64>>,
    is_trained: bool,
}

impl GaussianProcessRegressor {
    pub fn new(config: GPRConfig) -> Self {
        Self {
            config,
            train_x: Vec::new(),
            train_y: Vec::new(),
            k_matrix: Vec::new(),
            alpha: Vec::new(),
            l_cholesky: Vec::new(),
            is_trained: false,
        }
    }

    pub fn kernel(&self, x1: f64, x2: f64) -> f64 {
        match self.config.kernel_type.as_str() {
            "rbf" => {
                let dist = (x1 - x2).powi(2);
                self.config.signal_variance * (-dist / (2.0 * self.config.length_scale.powi(2))).exp()
            }
            "matern32" => {
                let dist = (x1 - x2).abs();
                let sqrt3 = 3.0_f64.sqrt();
                let arg = sqrt3 * dist / self.config.length_scale;
                self.config.signal_variance * (1.0 + arg) * (-arg).exp()
            }
            "matern52" => {
                let dist = (x1 - x2).abs();
                let sqrt5 = 5.0_f64.sqrt();
                let arg = sqrt5 * dist / self.config.length_scale;
                self.config.signal_variance
                    * (1.0 + arg + arg * arg / 3.0)
                    * (-arg).exp()
            }
            _ => {
                let dist = (x1 - x2).powi(2);
                self.config.signal_variance * (-dist / (2.0 * self.config.length_scale.powi(2))).exp()
            }
        }
    }

    pub fn fit(&mut self, x: &[f64], y: &[f64]) -> Result<(), &'static str> {
        if x.len() != y.len() {
            return Err("Input and output dimensions mismatch");
        }
        if x.len() < 2 {
            return Err("Need at least 2 training points");
        }

        let n = x.len();
        self.train_x = x.to_vec();
        self.train_y = y.to_vec();

        let mut k = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                k[i][j] = self.kernel(x[i], x[j]);
            }
            k[i][i] += self.config.noise_variance;
        }

        let l = cholesky_decomposition(&k).ok_or("Cholesky decomposition failed")?;

        let alpha = solve_cholesky(&l, y);

        self.k_matrix = k;
        self.l_cholesky = l;
        self.alpha = alpha;
        self.is_trained = true;

        Ok(())
    }

    pub fn predict(&self, x_test: &[f64]) -> Result<(Vec<f64>, Vec<f64>), &'static str> {
        if !self.is_trained {
            return Err("Model not trained");
        }

        let n_train = self.train_x.len();
        let n_test = x_test.len();

        let mut k_star = vec![vec![0.0; n_train]; n_test];
        for i in 0..n_test {
            for j in 0..n_train {
                k_star[i][j] = self.kernel(x_test[i], self.train_x[j]);
            }
        }

        let mut mean = vec![0.0; n_test];
        for i in 0..n_test {
            for j in 0..n_train {
                mean[i] += k_star[i][j] * self.alpha[j];
            }
        }

        let mut variance = vec![0.0; n_test];
        for i in 0..n_test {
            let k_ss = self.kernel(x_test[i], x_test[i]) + self.config.noise_variance;

            let v = solve_lower_triangular(&self.l_cholesky, &k_star[i]);

            let mut v_dot_v = 0.0;
            for k in 0..n_train {
                v_dot_v += v[k] * v[k];
            }

            variance[i] = (k_ss - v_dot_v).max(1e-10);
        }

        Ok((mean, variance))
    }

    pub fn predict_endpoint(&self) -> Result<GPRPredictionResult, &'static str> {
        if !self.is_trained {
            return Err("Model not trained");
        }

        let num_points = 200;
        let mut pred_times = Vec::with_capacity(num_points);
        let t_min = self.train_x[0];
        let t_max = self.config.max_prediction_hours.max(
            *self.train_x.last().unwrap() * 3.0
        );

        for i in 0..num_points {
            let t = t_min + (t_max - t_min) * i as f64 / (num_points - 1) as f64;
            pred_times.push(t);
        }

        let (mean, variance) = self.predict(&pred_times)?;
        let std_dev: Vec<f64> = variance.iter().map(|v| v.sqrt()).collect();

        let z = normal_quantile((1.0 + self.config.confidence_level) / 2.0);
        let lower: Vec<f64> = mean.iter().zip(std_dev.iter())
            .map(|(m, s)| m - z * s)
            .collect();
        let upper: Vec<f64> = mean.iter().zip(std_dev.iter())
            .map(|(m, s)| m + z * s)
            .collect();

        let target = self.config.target_moisture;
        let end_time_mean = find_crossing_time(&pred_times, &mean, target);
        let end_time_lower = find_crossing_time(&pred_times, &lower, target);
        let end_time_upper = find_crossing_time(&pred_times, &upper, target);

        let current_time = *self.train_x.last().unwrap();
        let remaining = end_time_mean - current_time;

        let remaining_lower = end_time_lower - current_time;
        let remaining_upper = end_time_upper - current_time;

        let r_squared = self.calculate_r_squared();
        let lml = self.calculate_log_marginal_likelihood();

        let target_idx = find_closest_index(&pred_times, end_time_mean);
        let uncertainty_at_target = if target_idx < std_dev.len() {
            std_dev[target_idx] * z
        } else {
            0.0
        };

        Ok(GPRPredictionResult {
            predicted_end_time_hours: end_time_mean,
            confidence_lower_hours: end_time_lower,
            confidence_upper_hours: end_time_upper,
            remaining_hours: remaining,
            confidence_interval_lower: remaining_lower,
            confidence_interval_upper: remaining_upper,
            predicted_curve_time: pred_times,
            predicted_curve_mean: mean,
            predicted_curve_lower: lower,
            predicted_curve_upper: upper,
            training_data_time: self.train_x.clone(),
            training_data_moisture: self.train_y.clone(),
            r_squared,
            log_marginal_likelihood: lml,
            uncertainty_at_target,
        })
    }

    fn calculate_r_squared(&self) -> f64 {
        let n = self.train_x.len();
        let y_mean: f64 = self.train_y.iter().sum::<f64>() / n as f64;

        let y_pred = match self.predict(&self.train_x) {
            Ok((m, _)) => m,
            Err(_) => return 0.0,
        };

        let mut ss_res = 0.0;
        let mut ss_tot = 0.0;
        for i in 0..n {
            ss_res += (self.train_y[i] - y_pred[i]).powi(2);
            ss_tot += (self.train_y[i] - y_mean).powi(2);
        }

        if ss_tot.abs() < 1e-10 {
            1.0
        } else {
            1.0 - ss_res / ss_tot
        }
    }

    fn calculate_log_marginal_likelihood(&self) -> f64 {
        if !self.is_trained {
            return 0.0;
        }

        let n = self.train_x.len();

        let y_alpha: f64 = self.train_y.iter()
            .zip(self.alpha.iter())
            .map(|(y, a)| y * a)
            .sum();

        let mut log_det = 0.0;
        for i in 0..n {
            log_det += self.l_cholesky[i][i].ln();
        }
        log_det *= 2.0;

        let pi2 = 2.0 * std::f64::consts::PI;
        -0.5 * (y_alpha + n as f64 * pi2.ln() + log_det)
    }

    pub fn optimize_hyperparameters(&mut self, x: &[f64], y: &[f64]) -> Result<(), &'static str> {
        let mut best_lml = f64::NEG_INFINITY;
        let mut best_params = (self.config.length_scale, self.config.signal_variance, self.config.noise_variance, self.config.kernel_type.clone());

        let length_scales = vec![10.0, 50.0, 100.0, 200.0, 500.0];
        let sigmas = vec![10.0, 25.0, 50.0, 100.0];
        let noises = vec![0.1, 0.5, 1.0, 2.0];
        let kernels = vec!["matern52", "matern32", "rbf"];

        for &kernel in &kernels {
            for &ls in &length_scales {
                for &sv in &sigmas {
                    for &nv in &noises {
                        self.config.kernel_type = kernel.to_string();
                        self.config.length_scale = ls;
                        self.config.signal_variance = sv;
                        self.config.noise_variance = nv;

                        if self.fit(x, y).is_ok() {
                            let lml = self.calculate_log_marginal_likelihood();
                            if lml > best_lml {
                                best_lml = lml;
                                best_params = (ls, sv, nv, kernel.to_string());
                            }
                        }
                    }
                }
            }
        }

        self.config.kernel_type = best_params.3;
        self.config.length_scale = best_params.0;
        self.config.signal_variance = best_params.1;
        self.config.noise_variance = best_params.2;
        self.fit(x, y)?;

        Ok(())
    }
}

fn cholesky_decomposition(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i][k] * l[j][k];
            }

            if i == j {
                let diag = a[i][i] - sum;
                if diag <= 0.0 {
                    return None;
                }
                l[i][j] = diag.sqrt();
            } else {
                l[i][j] = (a[i][j] - sum) / l[j][j];
            }
        }
    }

    Some(l)
}

fn solve_lower_triangular(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = l.len();
    let mut x = vec![0.0; n];

    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..i {
            sum += l[i][j] * x[j];
        }
        x[i] = (b[i] - sum) / l[i][i];
    }

    x
}

fn solve_upper_triangular(u: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let n = u.len();
    let mut x = vec![0.0; n];

    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += u[i][j] * x[j];
        }
        x[i] = (b[i] - sum) / u[i][i];
    }

    x
}

fn solve_cholesky(l: &[Vec<f64>], b: &[f64]) -> Vec<f64> {
    let y = solve_lower_triangular(l, b);
    let l_transposed = transpose(l);
    solve_upper_triangular(&l_transposed, &y)
}

fn transpose(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    let mut t = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            t[i][j] = m[j][i];
        }
    }
    t
}

fn find_crossing_time(times: &[f64], values: &[f64], target: f64) -> f64 {
    for i in 1..times.len() {
        if (values[i - 1] - target) * (values[i] - target) <= 0.0 {
            let t1 = times[i - 1];
            let t2 = times[i];
            let v1 = values[i - 1];
            let v2 = values[i];

            if (v2 - v1).abs() < 1e-10 {
                return t1;
            }

            return t1 + (target - v1) * (t2 - t1) / (v2 - v1);
        }
    }

    *times.last().unwrap()
}

fn find_closest_index(arr: &[f64], target: f64) -> usize {
    let mut best_idx = 0;
    let mut best_dist = f64::INFINITY;

    for (i, &val) in arr.iter().enumerate() {
        let dist = (val - target).abs();
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }

    best_idx
}

fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }

    if p < 0.5 {
        return -normal_quantile(1.0 - p);
    }

    let a = [2.50662823884, -18.61500062529, 41.39119773534, -25.44106049637];
    let b = [-8.47351093090, 23.08336743743, -21.06224101826, 3.13082909833];

    let q = p - 0.5;
    let r = q * q;

    let num = (((a[3] * r + a[2]) * r + a[1]) * r + a[0]) * q;
    let den = (((b[3] * r + b[2]) * r + b[1]) * r + b[0]) * r + 1.0;

    num / den
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_positive_definite() {
        let config = GPRConfig::default();
        let gpr = GaussianProcessRegressor::new(config);

        let k = gpr.kernel(1.0, 1.0);
        assert!(k > 0.0);
    }

    #[test]
    fn test_kernel_symmetry() {
        let config = GPRConfig::default();
        let gpr = GaussianProcessRegressor::new(config);

        let k1 = gpr.kernel(1.0, 3.0);
        let k2 = gpr.kernel(3.0, 1.0);
        assert!((k1 - k2).abs() < 1e-10);
    }

    #[test]
    fn test_fit_and_predict() {
        let config = GPRConfig {
            length_scale: 50.0,
            signal_variance: 100.0,
            noise_variance: 0.1,
            ..Default::default()
        };
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 24.0, 48.0, 72.0, 96.0];
        let y = vec![80.0, 72.0, 65.0, 58.0, 52.0];

        assert!(gpr.fit(&x, &y).is_ok());

        let x_test = vec![10.0, 50.0, 100.0];
        let (mean, variance) = gpr.predict(&x_test).unwrap();

        assert_eq!(mean.len(), 3);
        assert_eq!(variance.len(), 3);
        for &v in &variance {
            assert!(v >= 0.0);
        }
    }

    #[test]
    fn test_endpoint_prediction() {
        let config = GPRConfig {
            length_scale: 100.0,
            signal_variance: 100.0,
            noise_variance: 0.1,
            target_moisture: 15.0,
            max_prediction_hours: 2000.0,
            ..Default::default()
        };
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 50.0, 100.0, 200.0, 300.0];
        let y = vec![75.0, 60.0, 48.0, 30.0, 22.0];

        gpr.fit(&x, &y).unwrap();
        let result = gpr.predict_endpoint().unwrap();

        assert!(result.predicted_end_time_hours > *x.last().unwrap());
        assert!(result.remaining_hours > 0.0);
        assert!(result.confidence_lower_hours <= result.predicted_end_time_hours);
        assert!(result.predicted_end_time_hours <= result.confidence_upper_hours);
    }

    #[test]
    fn test_cholesky_decomposition() {
        let a = vec![
            vec![4.0, 2.0],
            vec![2.0, 5.0],
        ];
        let l = cholesky_decomposition(&a).unwrap();

        let mut a_reconstructed = vec![vec![0.0; 2]; 2];
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    a_reconstructed[i][j] += l[i][k] * l[j][k];
                }
            }
        }

        for i in 0..2 {
            for j in 0..2 {
                assert!((a_reconstructed[i][j] - a[i][j]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_optimize_hyperparameters() {
        let mut gpr = GaussianProcessRegressor::new(GPRConfig::default());

        let x = vec![0.0, 20.0, 50.0, 100.0, 150.0];
        let y = vec![80.0, 70.0, 55.0, 40.0, 30.0];

        let result = gpr.optimize_hyperparameters(&x, &y);
        assert!(result.is_ok());
        assert!(gpr.config.length_scale > 0.0);
    }

    #[test]
    fn test_normal_quantile() {
        let z95 = normal_quantile(0.975);
        assert!((z95 - 1.96).abs() < 0.01);

        let z50 = normal_quantile(0.5);
        assert!(z50.abs() < 1e-6);
    }

    #[test]
    fn test_prediction_interval_covers_true_endpoint() {
        let config = GPRConfig {
            length_scale: 80.0,
            signal_variance: 150.0,
            noise_variance: 0.05,
            target_moisture: 15.0,
            confidence_level: 0.95,
            max_prediction_hours: 2000.0,
            ..Default::default()
        };
        let mut gpr = GaussianProcessRegressor::new(config);

        let true_endpoint_hours = 450.0;
        let decay_rate = (75.0 - 15.0) / true_endpoint_hours;

        let x_train: Vec<f64> = (0..8).map(|i| i as f64 * 40.0).collect();
        let y_train: Vec<f64> = x_train.iter()
            .map(|&t| 15.0 + (75.0 - 15.0) * (-decay_rate * t / 60.0).exp())
            .collect();

        gpr.fit(&x_train, &y_train).unwrap();
        let result = gpr.predict_endpoint().unwrap();

        assert!(result.confidence_lower_hours <= true_endpoint_hours,
                "Lower bound {:.1} should be <= true endpoint {:.1}",
                result.confidence_lower_hours, true_endpoint_hours);
        assert!(result.confidence_upper_hours >= true_endpoint_hours,
                "Upper bound {:.1} should be >= true endpoint {:.1}",
                result.confidence_upper_hours, true_endpoint_hours);
        assert!(result.remaining_hours > 0.0);
    }

    #[test]
    fn test_multiple_kernel_functions() {
        let kernels = ["rbf", "matern32", "matern52"];

        let x = vec![0.0, 24.0, 48.0, 72.0, 96.0];
        let y = vec![80.0, 72.0, 65.0, 58.0, 52.0];
        let x_test = vec![50.0];

        for kernel in &kernels {
            let config = GPRConfig {
                kernel_type: kernel.to_string(),
                length_scale: 100.0,
                signal_variance: 100.0,
                noise_variance: 0.1,
                ..Default::default()
            };
            let mut gpr = GaussianProcessRegressor::new(config);
            assert!(gpr.fit(&x, &y).is_ok(), "Fit should succeed for {}", kernel);

            let (mean, var) = gpr.predict(&x_test).unwrap();
            assert!(mean[0].is_finite(), "Mean should be finite for {}", kernel);
            assert!(var[0] >= 0.0, "Variance should be non-negative for {}", kernel);
        }
    }

    #[test]
    fn test_minimal_training_data_boundary() {
        let config = GPRConfig::default();
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 48.0];
        let y = vec![80.0, 60.0];

        assert!(gpr.fit(&x, &y).is_ok());
        let (mean, _) = gpr.predict(&[24.0]).unwrap();
        assert!((mean[0] - 70.0).abs() < 5.0);
    }

    #[test]
    fn test_already_at_target_anomaly() {
        let config = GPRConfig {
            target_moisture: 15.0,
            ..Default::default()
        };
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 24.0, 48.0, 72.0];
        let y = vec![18.0, 16.0, 14.0, 12.0];

        gpr.fit(&x, &y).unwrap();
        let result = gpr.predict_endpoint().unwrap();

        assert!(result.predicted_end_time_hours > 0.0);
        assert!(result.predicted_end_time_hours < *x.last().unwrap() + 100.0,
                "If crossing happened near or before the last data point, endpoint should be reasonable");
    }

    #[test]
    fn test_non_monotonic_data_boundary() {
        let config = GPRConfig {
            target_moisture: 15.0,
            noise_variance: 1.0,
            ..Default::default()
        };
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 24.0, 48.0, 72.0, 96.0];
        let y = vec![80.0, 75.0, 78.0, 65.0, 55.0];

        assert!(gpr.fit(&x, &y).is_ok(), "Should handle non-monotonic noisy data");

        let result = gpr.predict_endpoint();
        assert!(result.is_ok(), "Endpoint prediction should not crash with noisy data");
    }

    #[test]
    fn test_extrapolation_far_future() {
        let config = GPRConfig {
            target_moisture: 15.0,
            max_prediction_hours: 5000.0,
            length_scale: 200.0,
            ..Default::default()
        };
        let max_hours = config.max_prediction_hours;
        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 50.0, 100.0];
        let y = vec![75.0, 70.0, 66.0];

        gpr.fit(&x, &y).unwrap();
        let result = gpr.predict_endpoint().unwrap();

        assert!(result.predicted_end_time_hours > *x.last().unwrap());
        assert!(result.predicted_end_time_hours <= max_hours);
        assert!(result.confidence_upper_hours > result.confidence_lower_hours);
    }

    #[test]
    fn test_inconsistent_input_lengths() {
        let mut gpr = GaussianProcessRegressor::new(GPRConfig::default());

        let x = vec![0.0, 24.0, 48.0];
        let y = vec![80.0, 70.0];

        let result = gpr.fit(&x, &y);
        assert!(result.is_err(), "Should fail with inconsistent input lengths");
    }

    #[test]
    fn test_confidence_level_boundary() {
        for &level in &[0.5, 0.9, 0.95, 0.99] {
            let config = GPRConfig {
                confidence_level: level,
                target_moisture: 15.0,
                length_scale: 100.0,
                signal_variance: 100.0,
                noise_variance: 0.1,
                ..Default::default()
            };
            let mut gpr = GaussianProcessRegressor::new(config);

            let x = vec![0.0, 50.0, 100.0, 150.0];
            let y = vec![75.0, 65.0, 55.0, 45.0];

            gpr.fit(&x, &y).unwrap();
            let result = gpr.predict_endpoint().unwrap();

            let interval_width = result.confidence_upper_hours - result.confidence_lower_hours;
            assert!(interval_width > 0.0,
                    "Interval width should be positive for confidence level {}", level);
        }
    }

    #[test]
    fn test_matern_default_kernel_and_optimization() {
        let config = GPRConfig::default();
        assert_eq!(config.kernel_type, "matern52",
                   "Default kernel should be Matern 5/2");

        let mut gpr = GaussianProcessRegressor::new(config);

        let x = vec![0.0, 30.0, 60.0, 100.0, 150.0, 200.0];
        let y = vec![78.0, 65.0, 53.0, 40.0, 30.0, 24.0];

        gpr.fit(&x, &y).unwrap();
        let result_matern = gpr.predict_endpoint().unwrap();
        assert!(result_matern.predicted_end_time_hours > 0.0);

        gpr.optimize_hyperparameters(&x, &y).unwrap();
        assert!(gpr.config.kernel_type == "matern52"
                || gpr.config.kernel_type == "matern32"
                || gpr.config.kernel_type == "rbf",
                "Optimized kernel should be one of the valid types, got {}", gpr.config.kernel_type);

        let result_optimized = gpr.predict_endpoint().unwrap();
        assert!(result_optimized.log_marginal_likelihood.is_finite());
        assert!(result_optimized.r_squared > 0.5,
                "R-squared should be reasonable after optimization");
    }
}
