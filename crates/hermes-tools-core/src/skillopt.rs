use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOptConfig {
    pub optimizer_backend: String,
    pub target_backend: String,
    pub optimizer_model: String,
    pub target_model: String,
    pub num_epochs: usize,
    pub batch_size: usize,
    pub train_size: usize,
    pub max_steps: usize,
    pub out_root: String,
    pub auto_adopt: bool,
}

impl Default for SkillOptConfig {
    fn default() -> Self {
        Self {
            optimizer_backend: "openai_compatible".into(),
            target_backend: "openai_compatible".into(),
            optimizer_model: "auto/best-reasoning".into(),
            target_model: "auto/cheap".into(),
            num_epochs: 1,
            batch_size: 2,
            train_size: 400,
            max_steps: 1,
            out_root: r"C:\Users\buzbe\OneDrive\Masaüstü\hermes yazılım\output\skillopt-smoke".into(),
            auto_adopt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardGateCheck {
    pub tdd_cycle_passed: bool,
    pub no_panic_result_passed: bool,
    pub opc_zip_order_passed: bool,
    pub equivalence_passed: bool,
    pub zero_clippy_warnings_passed: bool,
}

impl HardGateCheck {
    pub fn is_all_passed(&self) -> bool {
        self.tdd_cycle_passed
            && self.no_panic_result_passed
            && self.opc_zip_order_passed
            && self.equivalence_passed
            && self.zero_clippy_warnings_passed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftScoreMatrix {
    pub safety: f64,        // %25
    pub equivalence: f64,   // %20
    pub architecture: f64,  // %20
    pub performance: f64,   // %15
    pub clarity: f64,       // %10
    pub maintainability: f64, // %10
}

impl SoftScoreMatrix {
    pub fn calculate_total_score(&self) -> f64 {
        (self.safety * 0.25)
            + (self.equivalence * 0.20)
            + (self.architecture * 0.20)
            + (self.performance * 0.15)
            + (self.clarity * 0.10)
            + (self.maintainability * 0.10)
    }
}

pub fn generate_skillopt_command(config: &SkillOptConfig, config_yaml_path: &str) -> String {
    format!(
        "skillopt-train --config {} --optimizer_backend {} --target_backend {} --optimizer_model \"{}\" --target_model \"{}\" --num_epochs {} --batch_size {} --train_size {} --max_steps {} --out_root \"{}\"",
        config_yaml_path,
        config.optimizer_backend,
        config.target_backend,
        config.optimizer_model,
        config.target_model,
        config.num_epochs,
        config.batch_size,
        config.train_size,
        config.max_steps,
        config.out_root
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn hard_gate_ve_soft_score_hesabi() {
        let gate = HardGateCheck {
            tdd_cycle_passed: true,
            no_panic_result_passed: true,
            opc_zip_order_passed: true,
            equivalence_passed: true,
            zero_clippy_warnings_passed: true,
        };
        assert!(gate.is_all_passed());

        let score = SoftScoreMatrix {
            safety: 100.0,
            equivalence: 100.0,
            architecture: 100.0,
            performance: 100.0,
            clarity: 100.0,
            maintainability: 100.0,
        };
        assert_eq!(score.calculate_total_score(), 100.0);
    }
}
