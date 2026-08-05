# Простой скрипт чтобы избегать наличия несобирательного кода в репозитории.
# Этот скрипт выполняется когда код заливается в репозиторий. Этот скрипт более строгий чем
# `pre-commit.sh`. Тут присутствуют следующии минимальные требования:
# 1. Весь код должен быть правильно отформатирован с `cargo fmt --all`
# 2. Весь код должен быть собираться без ошибок. (в крайнем случае можно
#    отключить при большой спешке, но по умолчанию код должен собираться)
# 3. Все тесты должны проходить. (В крайнем случае можно отключить при
#    большой спешки, ибо лучше салить неготовый код чем всё потерять потому что
#    дом горит а комп. уже вынести время нет.)

#! /bin/sh

git diff --exit-code
sudo docker build --tag=uzor-docs-builder -f=Dockerfile || {
    echo "Uncommitted changes. For now please commit everything before pushing."
    exit 1
}

cargo fmt --all --check || {
    echo "Cannot push: Please push only fully formatted code!"
    exit 1
}

cargo clippy --all-targets || {
    echo "Cannot push: Lints do not pass."
    exit 1
}

cargo test || {
    echo "Cannot push: Tests do not pass"
    exit 1
}

echo "Pre-push check status: SUCCESS"
exit 0
