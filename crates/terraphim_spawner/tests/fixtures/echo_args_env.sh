#!/usr/bin/env bash
# Test fixture: records argv and selected env vars to $RECORD_FILE.
#
# Used by tests/spawn_task_body.rs to assert what the spawner actually
# delivers to the child process. The fixture itself is intentionally tiny
# so the test record file is easy to inspect on failure.
set -u

if [ -z "${RECORD_FILE:-}" ]; then
  echo "echo_args_env.sh: RECORD_FILE not set" >&2
  exit 2
fi

{
  printf 'argc=%d\n' "$#"
  for ((i = 1; i <= $#; i++)); do
    printf 'argv[%d]=%s\n' "$i" "${!i}"
  done
  printf 'ADF_TASK_SUMMARY=%s\n' "${ADF_TASK_SUMMARY:-<unset>}"
  printf 'ADF_FIXTURE_MARKER=%s\n' "${ADF_FIXTURE_MARKER:-<unset>}"
} > "$RECORD_FILE"
