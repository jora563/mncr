// Прототип пайплайна. Этот демо работает на локальной инстанции jenkins.
// Опираясь на этот пайплайн, строятся настоящие телеконтактовские пайплайны.
pipeline {
    // agent any
    environment {
        BBADDR = 'bitbucket.telecontact.ru/scm/telecontact/aiomni-core.git'
    //     DOCKER_REPO_URL             = "https://tc-dockerhub.telecontact.ru/v2/"
    //     DOCKER_REGISTRY_MIRROR      = 'tc-dockerhub.telecontact.ru/mirror/'
    //     DOCKER_RELEASE_REPO         = 'tc-dockerhub.telecontact.ru/'
    //     DOCKERFILE                  = 'pipeline/general.Dockerfile'
    }
    // agent any
    agent {
        docker {
            image 'aio-core-standalone:latest'
            args '-u root:root'
        }
    }
    options {
        disableConcurrentBuilds()
    }

    stages {
        stage('get-repo') {
            steps {
                echo 'clone libraries into container'
                sh "if ! [ -d aiomni-core/ ]; then     git clone 'https://${cred_name}:${cred_pw}@${BBADDR}'; fi"
                // sh 'rm -r telecontact-rust-libs'
                sh 'cd aiomni-core && git checkout feature/AIOMNI-9-ci-cd'
                sh 'ls -lash'
            }
        }
        stage('format') {
            steps {
                sh 'ls -lash'
                sh 'cd aiomni-core && ls -lash'
                echo 'Checking formatting of libraries..'
                sh 'cd aiomni-core && cargo fmt --all --check'
            }
        }
        stage('lint') {
            steps {
                echo 'Linting/building libraries..'
                sh 'cd aiomni-core && cargo clippy --all-targets'
            }
        }
        stage('tests-build') {
            steps {
                echo 'Building library tests..'
                sh 'cd aiomni-core && cargo test --no-run'
            }
        }
        stage('tests-run') {
            steps {
                echo 'Running library tests..'
                sh 'service postgresql start'
                sh 'cd aiomni-core && cargo test'
            }
        }
    }
    post {
        always {
            echo '`aiomni-core` basic pipeline finished. Running cleanup.'
        }
        aborted {
            echo "`aiomni-core` basic pipeline aborted. Cannot merge."
        }
        failure {
            echo "`aiomni-core` basic pipeline failed. Cannot merge."
        }
        success {
            echo "`aiomni-core` basic test pipeline successful. Jenkins allows merge."
        }
        cleanup {
            echo "Cleaning up pipeline.."
            // This script (and its maintenance) can be simplified by cleaning everything
            // and then we do not have to keep track of which packages need to be cleaned.
            sh '''
                cd aiomni-core
                cargo clean -p ai-omni-core -p chat -p db -p db_derive -p llm -p llm-client -p queue
            '''
            echo "..Cleaned up pipeline."
        }
    }
}
