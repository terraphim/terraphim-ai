fn main() {
    println!("fcctl-web server starting on http://127.0.0.1:8080");
    println!("Endpoints:");
    println!("  GET  /health");
    println!("  POST /api/vms");
    println!("  GET  /api/vms");
    println!("  DELETE /api/vms/{{vm_id}}");
    println!("  POST /api/vms/{{vm_id}}/upload");
    println!("  GET  /api/vms/{{vm_id}}/download");
    println!("\nNOTE: This is a placeholder implementation.");
    println!("Full implementation requires warp/axum for proper HTTP routing.");
    println!("For now, use the existing fcctl-web in terraphim_github_runner.");

    // Keep the process running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
