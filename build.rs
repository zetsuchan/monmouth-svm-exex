use std::env;
use std::path::Path;

fn main() {
    // Print build information
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=TARGET");
    
    // Get build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=BUILD_TARGET={}", target);
    
    // Set optimization flags for SVM processing
    if profile == "release" {
        println!("cargo:rustc-link-arg=-Wl,--strip-all");
        println!("cargo:rustc-env=SVM_OPTIMIZED=1");
    }
    
    // Check for required system dependencies
    check_system_deps();
    
    // Generate build timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now);
    
    // Set AI agent configuration
    if cfg!(feature = "ai-agents") {
        println!("cargo:rustc-env=AI_AGENTS_ENABLED=1");
        println!("cargo:rustc-cfg=ai_agents");
    }
    
    // Reth ExEx specific configuration
    println!("cargo:rustc-env=RETH_EXEX_VERSION=1.5.0");
    println!("cargo:rustc-env=SVM_VERSION=2.0");
}

fn check_system_deps() {
    // Check for required system libraries
    let deps = vec![
        ("pkg-config", "pkg-config"),
        ("clang", "clang"),
    ];
    
    for (lib, pkg) in deps {
        if !Path::new(&format!("/usr/bin/{}", lib)).exists() && 
           !Path::new(&format!("/usr/local/bin/{}", lib)).exists() {
            println!("cargo:warning=Missing system dependency: {}", pkg);
        }
    }
}