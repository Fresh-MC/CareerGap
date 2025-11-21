use nogap_core::policy_parser::load_policy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading policies from policies.yaml...");
    
    let policies = load_policy("policies.yaml")?;
    
    // Count policies by platform
    let windows_count = policies.iter().filter(|p| p.platform == "windows").count();
    let linux_count = policies.iter().filter(|p| p.platform == "linux").count();
    
    println!("\n✅ Successfully loaded {} total policies:", policies.len());
    println!("   - Windows policies: {}", windows_count);
    println!("   - Linux policies: {}", linux_count);
    
    // Show first 5 Linux policies
    println!("\n📋 First 5 Linux policies:");
    for (i, policy) in policies.iter().filter(|p| p.platform == "linux").take(5).enumerate() {
        println!("   {}. {} (ID: {})", i+1, 
            policy.title.as_deref().unwrap_or("N/A"), 
            policy.id.as_str());
        println!("      Severity: {}, Type: {}", 
            policy.severity.as_deref().unwrap_or("N/A"), 
            policy.check_type.as_str());
    }
    
    // Show policy breakdown by severity for Linux
    let linux_critical = policies.iter().filter(|p| 
        p.platform == "linux" && p.severity.as_deref() == Some("critical")).count();
    let linux_high = policies.iter().filter(|p| 
        p.platform == "linux" && p.severity.as_deref() == Some("high")).count();
    let linux_medium = policies.iter().filter(|p| 
        p.platform == "linux" && p.severity.as_deref() == Some("medium")).count();
    let linux_low = policies.iter().filter(|p| 
        p.platform == "linux" && p.severity.as_deref() == Some("low")).count();
    
    println!("\n📊 Linux policies by severity:");
    println!("   - Critical: {}", linux_critical);
    println!("   - High: {}", linux_high);
    println!("   - Medium: {}", linux_medium);
    println!("   - Low: {}", linux_low);
    
    Ok(())
}
