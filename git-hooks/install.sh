# Installs git hooks. Installation should be from the repo home.
#!/bin/bash

cp git-hooks/pre-commit.sh .git/hooks/pre-commit
cp git-hooks/pre-push.sh .git/hooks/pre-push
