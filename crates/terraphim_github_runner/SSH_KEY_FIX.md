# SSH Key Path Fix - Complete ✅

## Problem

Firecracker VM command execution was failing with SSH authentication errors:

```
Warning: Identity file ./images/test-vms/focal/keypair/fctest not accessible: No such file or directory.
Permission denied, please try again.
fctest@172.26.0.184: Permission denied (publickey,password).
```

### Root Cause

The `execute_command_via_ssh` function in `fcctl-web/src/api/llm.rs:281` was hardcoded to use focal SSH keys:
```rust
let ssh_key = "./images/test-vms/focal/keypair/fctest";
```

But `bionic-test` VMs use bionic keys located at:
```
./images/test-vms/bionic/keypair/fctest
```

## Solution

Modified `fcctl-web/src/api/llm.rs` to:

### 1. Capture VM Type Along with VM ID (lines 66-141)

Changed from:
```rust
let vm_id = if let Some(requested_vm_id) = payload.vm_id.clone() {
    // ... checks ...
    requested_vm_id
} else {
    // ... find vm ...
    vm.id
};
```

To:
```rust
let (vm_id, vm_type) = if let Some(requested_vm_id) = payload.vm_id.clone() {
    // ... checks ...
    (requested_vm_id, vm.vm_type)  // Return both ID and type
} else {
    // ... find vm ...
    (vm.id, vm.vm_type)  // Return both ID and type
};
```

### 2. Pass VM Type to SSH Function (line 173)

Changed from:
```rust
match execute_command_via_ssh(&vm_ip, &command).await {
```

To:
```rust
match execute_command_via_ssh(&vm_ip, &command, &vm_type).await {
```

### 3. Use Correct SSH Key Based on VM Type (lines 272-323)

Changed from:
```rust
async fn execute_command_via_ssh(
    vm_ip: &str,
    command: &str,
) -> Result<(String, String, i32), String> {
    // ...
    let ssh_key = "./images/test-vms/focal/keypair/fctest";  // Hardcoded
    // ...
}
```

To:
```rust
async fn execute_command_via_ssh(
    vm_ip: &str,
    command: &str,
    vm_type: &str,  // New parameter
) -> Result<(String, String, i32), String> {
    // ...
    // Determine SSH key path based on VM type
    let ssh_key = if vm_type.contains("bionic") {
        "./images/test-vms/bionic/keypair/fctest"
    } else if vm_type.contains("focal") {
        "./images/test-vms/focal/keypair/fctest"
    } else {
        // Default to focal for unknown types
        "./images/test-vms/focal/keypair/fctest"
    };

    info!("Using SSH key: {} for VM type: {}", ssh_key, vm_type);
    // ...
}
```

## Test Results

### Test 1: Echo Command
```json
{
  "execution_id": "e5207df6-8894-453c-a142-c3ddac85e23f",
  "vm_id": "vm-4062b151",
  "exit_code": 0,
  "stdout": "Hello from Firecracker VM!\n",
  "stderr": "Warning: Permanently added '172.26.0.230' (ECDSA) to the list of known hosts.\r\n",
  "duration_ms": 127,
  "started_at": "2025-12-25T11:03:58.611473106Z",
  "completed_at": "2025-12-25T11:03:58.738825817Z",
  "error": null
}
```

✅ **exit_code: 0**
✅ **stdout: "Hello from Firecracker VM!"**

### Test 2: List Files
```json
{
  "execution_id": "0fce5a50-8b05-4116-a5da-328f6568c560",
  "vm_id": "vm-4062b151",
  "exit_code": 0,
  "stdout": "total 28\ndrwxrwxrwt  7 root root 4096 Dec 25 10:50 .\ndrwxr-xr-x 22 root root 4096 Dec 25 00:09 ..\ndrwxrwxrwt  2 root root 4096 Dec 25 10:50 .ICE-unix\n...",
  "stderr": "Warning: Permanently added '172.26.0.230' (ECDSA) to the list of known hosts.\r\n",
  "duration_ms": 115
}
```

✅ **exit_code: 0**
✅ **stdout: Directory listing successful**

### Test 3: Check User
```json
{
  "execution_id": "f485b9d7-e229-4c3c-8721-6af3524bd015",
  "vm_id": "vm-4062b151",
  "exit_code": 0,
  "stdout": "fctest\n",
  "stderr": "Warning: Permanently added '172.26.0.230' (ECDSA) to the list of known hosts.\r\n",
  "duration_ms": 140
}
```

✅ **exit_code: 0**
✅ **stdout: Running as 'fctest' user**

## Verification Commands

```bash
# Build fcctl-web with fix
cd /home/alex/projects/terraphim/firecracker-rust
cargo build --release -p fcctl-web

# Restart service
sudo systemctl restart fcctl-web

# Create VM
JWT="<your-jwt-token>"
curl -s -X POST http://127.0.0.1:8080/api/vms \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"name":"test-vm","vm_type":"bionic-test"}'

# Execute command
VM_ID="<vm-id-from-response>"
curl -s -X POST http://127.0.0.1:8080/api/llm/execute \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"agent_id\":\"test\",
    \"language\":\"bash\",
    \"code\":\"echo 'Hello from VM!'\",
    \"vm_id\":\"$VM_ID\",
    \"timeout_seconds\":5,
    \"working_dir\":\"/tmp\"
  }"
```

## Summary

✅ **SSH key path FIXED**
- Commands now execute successfully in bionic-test VMs
- Correct SSH key is automatically selected based on VM type
- All tests passing with exit_code 0
- Full integration proven: API → VM selection → SSH → command execution

## Files Modified

| File | Changes |
|------|---------|
| `fcctl-web/src/api/llm.rs` | Lines 66-141: Capture vm_type with vm_id |
| `fcctl-web/src/api/llm.rs` | Line 173: Pass vm_type to SSH function |
| `fcctl-web/src/api/llm.rs` | Lines 272-323: Use correct SSH key based on vm_type |

---

*Fixed: 2025-12-25*
*All command execution tests passing*
