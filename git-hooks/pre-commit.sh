# Простой скрипт чтобы избегать наличия несобирательного кода в репозитории.
# Этот скрипт выполняется перед каждом комитом. Тут присутствуют следующии
# минимальные требования:
# 1. Весь код должен быть правильно отформатирован с `cargo fmt --all`
# 2. Весь код должен быть собираться без ошибок. (в крайнем случае можно
#    отключить при большой спешке, но по умолчанию код должен собираться)

#! /bin/sh

git diff --exit-code || {
    echo "Uncommitted changes. For now please commit everything before pushing."
    exit 1
fi

cargo fmt --all --check || {
    echo "Cannot push: Please push only fully formatted code!"
    exit 1
fi

cargo clippy --all-targets || {
    echo "Cannot push: Lints do not pass."
    exit 1
}

echo "Pre-commit check status: SUCCESS"
exit 0
