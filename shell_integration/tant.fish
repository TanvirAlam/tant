# Tant Terminal Shell Integration for Fish
# This script provides Warp-style command block detection using OSC 133 sequences

# Only run in interactive mode
status is-interactive; or exit 0

# Check if already loaded to prevent double-loading
if set -q TANT_SHELL_INTEGRATION
    exit 0
end
set -gx TANT_SHELL_INTEGRATION 1

# OSC 133 sequences for shell integration
# A = prompt start
# B = prompt end (command line start)
# C = command start (pre-execution)
# D = command end (pre-prompt, includes exit code)

# Function to emit OSC sequences
function _tant_osc
    printf "\033]133;%s\007" $argv[1]
end

# Emit git info (branch + status) using OSC 133;G
function _tant_emit_git_info
    set -l git_info (git -C . status --porcelain=v2 -b 2>/dev/null)
    or return 0
    set -l branch (printf "%s\n" "$git_info" | command awk '/^# branch.head /{print $3; exit}')
    if test -z "$branch"; or test "$branch" = "(detached)"
        return 0
    end

    set -l git_status clean
    printf "%s\n" "$git_info" | command awk '($1 == "u" || $1 == "?" || $1 == "1" || $1 == "2") {exit 0} END {exit 1}'
    if test $status -eq 0
        set git_status dirty
    end
    printf "%s\n" "$git_info" | command awk '$1 == "u" {exit 0} END {exit 1}'
    if test $status -eq 0
        set git_status conflicts
    end

    _tant_osc "G;branch=$branch;status=$git_status"
end

# Pre-execution hook: runs right before command execution
function _tant_preexec --on-event fish_preexec
    # Emit command start marker
    _tant_osc "C"
end

# Post-execution hook: runs after command completes
function _tant_postexec --on-event fish_postexec
    # Emit command end marker with exit code
    _tant_osc "D;$status"
end

# Prompt start hook: runs before displaying prompt
function _tant_prompt_start --on-event fish_prompt
    # Emit prompt start marker
    _tant_osc "A"
    _tant_emit_git_info
end

# Modify fish_prompt to include markers
# We need to wrap the existing prompt function
if not functions -q _tant_original_fish_prompt
    # Save the original prompt function
    functions -c fish_prompt _tant_original_fish_prompt
    
    # Create new prompt function with markers
    function fish_prompt
        # Emit prompt start
        _tant_osc "A"
        
        # Call original prompt
        _tant_original_fish_prompt
        
        # Emit prompt end (ready for command input)
        _tant_osc "B"
    end
end

# Hook into command execution
function _tant_preexec_handler --on-event fish_preexec
    _tant_osc "C"
end

function _tant_postexec_handler --on-event fish_postexec
    set -l exit_code $status
    _tant_osc "D;$exit_code"
end

# Emit initial prompt start marker
_tant_osc "A"

echo "Tant shell integration loaded for fish"
