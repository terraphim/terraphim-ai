# $(cat <<'EOF'

Quoted heredoc delimiter in command substitution prevents shell expansion of variables ($), backticks, and special characters inside the heredoc body. Unquoted <<EOF causes the shell to interpret $variables, `backtick commands`, and escape sequences, which mangles commit messages, curl bodies, and multi-line strings.

synonyms:: $(cat <<EOF