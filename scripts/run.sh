#!/bin/bash

# Define the directories to exclude (crates and bots)
EXCLUDE_DIRS=("crates" "bots")

# Function to check if a directory should be excluded
should_exclude() {
  local dir="$1"
  for exclude in "${EXCLUDE_DIRS[@]}"; do
    if [[ "$dir" == "$exclude" ]]; then
      return 0  # Exclude
    fi
  done
  return 1  # Not excluded
}

# Array to track background processes started by the script
PID_LIST=()

# Function to clean up running processes when exiting
cleanup() {
  echo "Cleaning up..."
  for pid in "${PID_LIST[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      echo "Terminating process with PID $pid"
      kill "$pid"
    fi
  done
  exit 0
}

# Trap to ensure cleanup happens on script exit
trap cleanup EXIT

# Loop through all directories
for dir in */; do
  if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ] && ! should_exclude "$dir"; then
    echo "Running project in directory: $dir"
    
    # Navigate into the project directory
    cd "$dir" || exit

    # Build the project
    echo "Building $dir..."
    cargo build &  # Run build in the background

    # Capture the PID of the build process
    PID_LIST+=($!)

    # Wait for the build to complete
    wait $!

    # Run the project
    echo "Running $dir..."
    cargo run &  # Run in background so script can continue

    # Capture the PID of the run process
    PID_LIST+=($!)

    # Wait for the current project to finish
    wait $!  # Wait for the current project to finish

    # Return to the root directory after processing the current project
    cd ..
  fi
done
