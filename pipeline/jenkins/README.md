# Объяснение пайплайна.

Существуют три файла.

- [local.groovy](local.groovy)
- [genericWebhookTriggerMerge-tc.groovy](genericWebhookTriggerMerge-tc.groovy)
- [postmerge.groovy](postmerge.groovy)

Они основаны на процессе "Telecontact Rust Libs" и работают в процессе проверок качества и работоспособности кода который заливают в репозиторий.

___
## Как пользоваться и что учесть.

Эта серия файлов описывает процесс сборки проекта aiomni-core на Jenkins. Файл [local.groovy](local.groovy) можно использовать на локальной инстанции Jenkins где отсутствует сложная корпоративная логика разграничения, и где сам Jenkins не запускается внутри контейнера. Также этот файл работает в упрощённом режиме, не реагирует на разные триггеры и берёт пользователя и пароль обычной строкой. On существует для проверки концепции.

В отличии от этого, [`genericWebhookTriggerMerge-tc.groovy`](genericWebhookTriggerMerge-tc.groovy) заточен на работу на инстанции телеконтакта, tc-jenkins-03.telecontact.ru для проверок CI при вливании одной ветку в другую. Во первых, эта рабочая инстанция псевдо-крупной коммерческой организации где работают строгие разграничения. Во вторых тут докер работает в докере (docker-in-docker). В третьих, пароли передаются в кёш самого Jenkins и вставляются безопасным способом. В четвертых, сам функционал заточен на интеграцию в инстанцию bitbucket через "вебхуки". В пятых, так как это файл для "настоящего пайплайна, вывод идёт не только на консоль, с полным описанием ошибок, но и в специальны лог файл.

___
## Дополнительные требования `genericWebhookTriggerMerge-tc.groovy`

Для работы [`genericWebhookTriggerMerge-tc.groovy`](genericWebhookTriggerMerge-tc.groovy) нужна интеграция в пайплайн `bitbucket`. Для этого надо чтобы был бот-пользователь на `bitbucket` и также нужен вспомогательный скрипт который по хукам меняет статус PR/MR при когда родственные ПР/МРы, для запроса перепроверки сборки. Этим скриптом является [`postmerge.groovy`](postmerge.groovy). Taк же нужен докер образ (или их набор) на котором возможно запустить сборку проекта.

___
### Переменные и настройки пайп-лайн `genericWebhookTriggerMerge-tc.groovy`

- Name: ai-omni-ci-pipeline
- Triggers: Generic Webhook Trigger
  - Variable: mr_id
    - Expression: $.pullRequest.id
    - JSONPath
  - Variable: incoming_branch
    - Expression: $.pullRequest.fromRef.displayId
    - JSONPath
  - Variable: receiving_branch
    - Expression: $.pullRequest.toRef.displayId
    - JSONPath
  - Variable: incoming_hash
    - Expression: $.pullRequest.fromRef.latestCommit
    - JSONPath
  - Variable: receiving_hash
    - Expression: $.pullRequest.toRef.latestCommit
    - JSONPath
  - Variable: mr_status
    - Expression: $.participant.status
    - JSONPath
  - Variable: needs_work_from
    - Expression: $.participant.user.name
    - JSONPath
- Token: aiomni-core-ci
- Definition: Pipeline script
  - Script: [`genericWebhookTriggerMerge-tc.groovy`](genericWebhookTriggerMerge-tc.groovy)

___
### Переменные и настройки пайп-лайн `postmerge.groovy`

- Name: ai-omni-uncheck-on-merge
- Triggers: Generic Webhook Trigger
  - Variable: mr_id
    - Expression: $.pullRequest.id
    - JSONPath
  - Variable: receiving_branch
    - Expression: $.pullRequest.toRef.displayId
    - JSONPath
  - Variable: incoming_branch
    - Expression: $.pullRequest.fromRef.displayId
    - JSONPath
  - Variable: current_branch
    - Expression: $.changed.ref.displayId
    - JSONPath
- Token: ai-omni-uncheck-on-merge
- Definition: Pipeline script
  - Script: ([`postmerge.groovy`](postmerge.groovy))

___
### Дополнительные требования со стороны Bitbucket

Со стороны bitbucket нужно настроить два хука типа "Webhooks" (Repository settings > Webhooks):

- Для `genericWebhookTriggerMerge-tc.groovy` хук настроен на "Event: PullRequest (Opened, Source branch updated, modified, needs work`).
  - Адрес: <https://tc-jenkins-03.telecontact.ru/generic-webhook-trigger/invoke?token=aiomni-core-ci>
- Для `postmerge.groovy` хук на "Event: PullRequest (Merged)".
  - Aдрес: <https://tc-jenkins-03.telecontact.ru/generic-webhook-trigger/invoke?token=ai-omni-uncheck-on-merge>

Дальше, эти скрипты уже на стороне Jenkins берут значения данных которые передаёт bitbucket. Скрипты должны быть настроены на "Generic Webhook Trigger", и брать "JSONPath" по инструкциям из самих `groovy` файлов.

Также надо проставить в разделе "Default Reviewers" (Respository settings > Default reviewers) пользователя "jenkins" как стандартного рецензента на все PR.

Также желательно, но не обязательно для работы пайп-лайн, ставить "Approvals required" на 1 (или больше) на влив в `development` и `master`. Также имеет смысл запретить заливать когда стоит значок "needs work" и когда есть по PR открытые задачи (но это скорее относится к хорошей практике, а не к работе пайп-лайн).

Для более детального разбора следует почитать документацию в самих `groovy` файлах.
