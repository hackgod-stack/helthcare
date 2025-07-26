use candid::{CandidType, Deserialize};
use ic_cdk::api::management_canister::main::raw_rand;
use ic_cdk_macros::*;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use k256::ecdsa::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct MedicalQuery {
    pub patient_id: String,
    pub symptoms: Vec<String>,
    pub medical_history: Vec<String>,
    pub timestamp: u64,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct DiagnosisResult {
    pub diagnosis: String,
    pub confidence: f64,
    pub recommendations: Vec<String>,
    pub risk_factors: Vec<String>,
    pub model_version: String,
    pub signature: Vec<u8>,
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct ModelWeights {
    pub version: String,
    pub weights: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub threshold_signature: Vec<u8>,
}

thread_local! {
    static MODEL_WEIGHTS: RefCell<Option<ModelWeights>> = RefCell::new(None);
    static SIGNING_KEY: RefCell<Option<SigningKey>> = RefCell::new(None);
}

#[init]
fn init() {
    ic_cdk::println!("AI Inference Canister initialized");
    
    // Initialize threshold-ECDSA signing key
    ic_cdk::spawn(async {
        match initialize_threshold_ecdsa().await {
            Ok(_) => ic_cdk::println!("Threshold-ECDSA initialized successfully"),
            Err(e) => ic_cdk::println!("Failed to initialize threshold-ECDSA: {:?}", e),
        }
    });
}

async fn initialize_threshold_ecdsa() -> Result<(), String> {
    // Generate random seed for threshold-ECDSA
    let (random_bytes,) = raw_rand().await.map_err(|e| format!("Failed to get random bytes: {:?}", e))?;
    
    // Create signing key from random bytes
    let signing_key = SigningKey::from_bytes(&random_bytes[..32])
        .map_err(|e| format!("Failed to create signing key: {:?}", e))?;
    
    SIGNING_KEY.with(|key| {
        *key.borrow_mut() = Some(signing_key);
    });
    
    Ok(())
}

#[update]
fn update_model_weights(weights: ModelWeights) -> Result<String, String> {
    // Verify threshold signature before updating
    if !verify_threshold_signature(&weights) {
        return Err("Invalid threshold signature".to_string());
    }
    
    MODEL_WEIGHTS.with(|model| {
        *model.borrow_mut() = Some(weights.clone());
    });
    
    ic_cdk::println!("Model weights updated to version: {}", weights.version);
    Ok(format!("Model updated to version: {}", weights.version))
}

#[query]
fn get_model_version() -> Option<String> {
    MODEL_WEIGHTS.with(|model| {
        model.borrow().as_ref().map(|m| m.version.clone())
    })
}

#[update]
async fn diagnose(query: MedicalQuery) -> Result<DiagnosisResult, String> {
    let model = MODEL_WEIGHTS.with(|m| m.borrow().clone());
    
    let model_weights = model.ok_or("No model weights loaded")?;
    
    // Simulate AI inference (in production, this would use the actual model)
    let diagnosis_result = perform_inference(&query, &model_weights).await?;
    
    // Sign the result with threshold-ECDSA
    let signed_result = sign_diagnosis_result(diagnosis_result).await?;
    
    Ok(signed_result)
}

async fn perform_inference(query: &MedicalQuery, _weights: &ModelWeights) -> Result<DiagnosisResult, String> {
    // This is a simplified simulation of AI inference
    // In production, this would use candle-core for actual neural network inference
    
    let diagnosis = if query.symptoms.contains(&"fever".to_string()) && 
                      query.symptoms.contains(&"cough".to_string()) {
        "Possible respiratory infection"
    } else if query.symptoms.contains(&"chest_pain".to_string()) {
        "Requires cardiac evaluation"
    } else {
        "General consultation recommended"
    };
    
    let confidence = 0.85; // Simulated confidence score
    
    Ok(DiagnosisResult {
        diagnosis: diagnosis.to_string(),
        confidence,
        recommendations: vec![
            "Consult with healthcare provider".to_string(),
            "Monitor symptoms".to_string(),
        ],
        risk_factors: vec!["Age".to_string(), "Medical history".to_string()],
        model_version: _weights.version.clone(),
        signature: vec![], // Will be filled by sign_diagnosis_result
    })
}

async fn sign_diagnosis_result(mut result: DiagnosisResult) -> Result<DiagnosisResult, String> {
    let signing_key = SIGNING_KEY.with(|key| key.borrow().clone());
    let key = signing_key.ok_or("Signing key not initialized")?;
    
    // Create hash of the diagnosis result
    let mut hasher = Sha256::new();
    hasher.update(result.diagnosis.as_bytes());
    hasher.update(&result.confidence.to_be_bytes());
    hasher.update(result.model_version.as_bytes());
    let hash = hasher.finalize();
    
    // Sign the hash (simplified - in production would use proper threshold-ECDSA)
    let signature = key.sign_prehash(&hash)
        .map_err(|e| format!("Failed to sign result: {:?}", e))?;
    
    result.signature = signature.to_bytes().to_vec();
    Ok(result)
}

fn verify_threshold_signature(weights: &ModelWeights) -> bool {
    // Simplified signature verification
    // In production, this would verify the threshold signature from federated learning
    !weights.threshold_signature.is_empty()
}

#[query]
fn get_canister_status() -> HashMap<String, String> {
    let mut status = HashMap::new();
    status.insert("status".to_string(), "active".to_string());
    status.insert("model_loaded".to_string(), 
                 MODEL_WEIGHTS.with(|m| m.borrow().is_some().to_string()));
    status.insert("threshold_ecdsa".to_string(), 
                 SIGNING_KEY.with(|k| k.borrow().is_some().to_string()));
    status
}

// Export Candid interface
ic_cdk::export_candid!();