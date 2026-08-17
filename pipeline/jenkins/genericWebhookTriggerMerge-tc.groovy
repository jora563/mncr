// НБ: This is the version of the pipeline to run on the "proper" telecontact Jenkins.
// It exists to carry out pre-merge build checks and allow a "bot user"
// to pre-approve the PR if the pre-merge test build is successful.
// The steps it carries out are as follows:
//
// 1. The bot unapproves the PR through the bitbucket REST API.
// 2. Jenkins pulls the repository and performs the merge in a docker container.
// 3. Jenkins lints, builds and tests the libraries (with postgres and mysql included)
// 4. If the build is successful, the bot approves the PR through the bitbucket REST API.
//
// (It is assumed that the approval of the bot user is only one of the necessary steps for approval)
//
// For Bitbucket 7.2:
//
// The trigger url is: https://tc-jenkins-03.telecontact.ru/generic-webhook-trigger/invoke?token=aiomni-core-ci
//
// For bitbucket 7.2 URL variables are not available, therefore in the Generic Webhook Trigger
// plugin for Jenkins we define all variables as body variables as follows:
//
// post: mr_id = $.pullRequest.id
// request: incoming_branch = $.pullRequest.fromRef.displayId
// request: receiving_branch = $.pullRequest.toRef.displayId
// request: incoming_hash = $.pullRequest.fromRef.latestCommit
// request: receiving_hash = $.participant.user.name (none)
// request: mr_status = $.participant.status (not_needs_work)
// request: needs_work_from = $.participant.user.name (unknown)
//
// The only required Jenkins plugin (if using Jenkins 2.528.1) is Generic Webhook Trigger.
// No plugins are required for bitbucket.
//
// НБ: Depending on the docker configurations the addresses need to be adjusted.

// Since we define the username and pw as `UN` and `PW`, they must be referred to in the called
// string as `${UN}` and `${PW}` respectively, eg `"curl --url \"http://${UN}:${PW}@host.com/index\""``
def callWithCreds(String user, String called) throws Exception {
    withCredentials([usernamePassword(
                    credentialsId: user,
                    usernameVariable: 'UN',
                    passwordVariable: 'PW'
                )]) {
                    sh called
                }
}


// This function saves the whole error message to a file and exits the script
// with a fail. The error stored in the FINAL_ERROR_FILE can then be retrieved
// and sent to Bitbucket to display as a comment from the builder.
def sendError(String kind, String e) {
    String last_msg = sh(script: "cat ${RAW_ERROR_FILE}", returnStdout: true).trim()
    echo "Error to Report: ${last_msg}"
    sh "echo \"${kind}: ${e}\" >${FINAL_ERROR_FILE}"
    sh '''
       set -x
       cat ${RAW_ERROR_FILE} >>${FINAL_ERROR_FILE}
    '''
    sh "exit 1"
}

// This allows multiline commands to be called, with each one being channelled into
// the error file.
def shWithErr(String called) {
    called = called.trim()
    sh "(${called}) 1>${RAW_ERROR_FILE} 2>${RAW_ERROR_FILE}"
    sh 'cat ${RAW_ERROR_FILE}'
}

// Send the build status to bitbucket.
// NB: Build status has a dual key:
//     - The commit in the URL
//     - the "key" field in the data
//
// It is possible to override the build data for a given commit with
// a new status if the key is the same. However, if the branches are updated
// and the commit changes, we may (or may not) end up with multiple builds, of
// which one fails, so this should be used more for information than for definitive
// permissions for merging a PR.
def sendBBBStatus(String status, String msg) throws Exception {
    echo "Sending to build api: ${status}"
    def now = new java.util.Date().getTime()
    echo "time: ${now}"
    withCredentials([usernamePassword(
                    credentialsId:  JENKINS,
                    usernameVariable: 'UN',
                    passwordVariable: 'PW'
                )]) {
        sh """
            curl --request POST \
            --header 'Accept: application/json;charset=UTF-8' \
            --header 'Content-Type: application/json' \
            --data '{ \
                "state": "${status}", \
                "name": "${BUILD_NUMBER}", \
                "url": "${BUILD_URL}", \
                "description": "Build ${status}", \
                "updatedDate": ${now}, \
                "createdDate": ${now}, \
                "key": "${incoming_branch}->${receiving_branch}" \
            }' \
            --user ${UN}:${PW} \
            --url "${HTTP}://${BBHOST}/rest/build-status/latest/commits/${incoming_hash}"
        """
    }
}

// This is used to avoid triggering builds from events when users other than Jenkins
// mark the PR with "needs work"
def needsWorkFromNotJenkins() {
    def ret = withCredentials([usernamePassword(
                    credentialsId:  JENKINS,
                    usernameVariable: 'UN',
                    passwordVariable: 'PW'
                )]) {
        if (needs_work_from != "${UN}" && mr_status == 'NEEDS_WORK') {
        echo "${needs_work_from}: Returning true"
           return true
        } else {
           return false
        }
    }
}

pipeline {
    environment {
        BBHOST = 'bitbucket.telecontact.ru'
        BBPROJ = 'telecontact'
        BBREPO = 'aiomni-core'
        BBADDR = "${BBHOST}/scm/${BBPROJ}/${BBREPO}.git"
        DOCKER_REPO_URL = 'tc-dockerhub.telecontact.ru/'
        DOCKER_IMAGE = 'tc-dockerhub.telecontact.ru/ai-omni/core_standalone:v0.4'
        JENKINS = 'jenkins_username_password'
        DOCKER_REPO_CREDENTIALS_ID = 'tc-dockerhub.telecontact.ru_buildmaker'
        HTTP = 'https'
        HTTP_PROXY = 'http://tc-msk-prxapp.telecontact.ru:8080'
        no_proxy = '127.0.0.53,127.0.0.1,bitbucket.telecontact.ru,stash.telecontact.ru,tc-chat.telecontact.ru,tc-dockerhub.telecontact.ru,tc-jenkins-03.telecontact.ru,tc-mirror.telecontact.ru,tc-plexus.telecontact.ru,tc-repo-composer.telecontact.ru,tc-repo.telecontact.ru'
        DOCKER_ARGS = "-u root:root --privileged --expose 80 --expose 443 --expose 8080 --expose 8081  --env no_proxy=${no_proxy} --env HTTP_PROXY=${HTTP_PROXY} --env HTTPS_PROXY=${HTTP_PROXY} --env http_proxy=${HTTP_PROXY} --env https_proxy=${HTTP_PROXY}"
        //DOCKER_ARGS = "-u root:root --privileged --expose 80 --expose 443 --expose 8080  --network host"
        RAW_ERROR_FILE = 'raw_error_file'
        FINAL_ERROR_FILE = 'final_error_file'
    }
    agent any
    // agent {
    //     node {
    //         label 'agent_test'
    //     }
    // }
    options {
        disableConcurrentBuilds()
    }
    stages {
        // We first unapprove the request, since it is being built!
        // We also check that our docker container is fresh and ready to go.
        stage('unapprove') {
            steps {
                script {
                    try {
                        // Skip everything if triggered by a NEEDS_WORK by not jenkins
                        if (this.needsWorkFromNotJenkins() == true) {
                            echo 'Not jenkins asked for a review'
                            shWithErr('exit 1')
                        }
                        echo "unapproving PR no.${mr_id} (${incoming_branch} -> ${receiving_branch}) and testing docker container..."
                        // Ensure that files exist, wipe them and recreate them.
                        sh "touch ${RAW_ERROR_FILE} && rm ${RAW_ERROR_FILE}  && touch ${RAW_ERROR_FILE}"
                        sh "touch ${FINAL_ERROR_FILE} && rm ${FINAL_ERROR_FILE}  && touch ${FINAL_ERROR_FILE}"
                        this.callWithCreds(
                            JENKINS,
                            '''
                                curl --request PUT \
                                --header 'Accept: application/json;charset=UTF-8' \
                                --header 'Content-Type: application/json' \
                                --data '{"status":"UNAPPROVED"}' \
                                --user ${UN}:${PW} \
                                --url "https://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests/${mr_id}/participants/jenkins"'''
                        )
                        this.sendBBBStatus('INPROGRESS', 'Build in progress')
                        shWithErr('curl tc-dockerhub.telecontact.ru')
                        withDockerRegistry([
                            credentialsId: "${DOCKER_REPO_CREDENTIALS_ID}",
                            url: "${HTTP}://${DOCKER_REPO_URL}"
                        ]) {
                            echo "Pulling docker image (${DOCKER_IMAGE})"
                            def img = docker.image("${DOCKER_IMAGE}")
                            img.pull()
                            echo "Pulled image successfully"
                        }
                    } catch (Exception e) {
                        sendError("Basic error", e.toString())
                    }
                }
            }
        }
        stage('get-repo') {
            agent {
                docker {
                    image "${DOCKER_IMAGE}"
                    args "${DOCKER_ARGS}"
                    registryUrl "${HTTP}://${DOCKER_REPO_URL}"
                    registryCredentialsId "${DOCKER_REPO_CREDENTIALS_ID}"
                    reuseNode true
                }
            }
            steps {
                echo 'clone libraries into container'
                // Conditionally clone the repository.
                // NB: We don't need to delete the library each time, but due to the whole build system
                // being tested on a single machine, this was the simplest temporary solution.
                script {
                    try {
                        shWithErr('curl tc-dockerhub.telecontact.ru')
                        this.callWithCreds(
                            JENKINS,
                            '''
                                if ! [ -d aiomni-core/ ]
                                then
                                    echo 'cloning into aiomni-core/'
                                    git clone ${HTTP}://${UN}:${PW}@${BBADDR}
                                else
                                    echo 'removing and cloning into aiomni-core/'
                                    if [ -d aiomni-core/target/ ]
                                    then
                                        mv aiomni-core/target target
                                    fi
                                    rm -r aiomni-core/
                                    git clone ${HTTP}://${UN}:${PW}@${BBADDR}
                                    if [ -d target/ ]
                                    then
                                        mv target aiomni-core/target
                                    fi
                                fi'''
                        )
                        shWithErr('cat /etc/resolv.conf')
                        shWithErr('curl https://index.crates.io/config.json')
                        shWithErr('''ls''')
                        // update the repository (if it exists wemust do this)
                        shWithErr('''
                            cd aiomni-core
                            git reset --hard
                            git checkout master
                            git pull''')
                        // Checkout the incoming branch and update it (it might exist)
                        shWithErr('''
                            cd aiomni-core
                            git checkout ${incoming_branch}
                            git pull''')
                        // Checkout the target branch and update it (it might exist)
                        shWithErr('''
                            cd aiomni-core
                            git checkout ${receiving_branch}
                            git pull''')
                        // Merge the branches (we need to test the final branch, not the source branch).
                        shWithErr('''
                            cd aiomni-core
                            git branch ${receiving_branch}_${incoming_branch}_test_merge_${BUILD_NUMBER}
                            git checkout ${receiving_branch}_${incoming_branch}_test_merge_${BUILD_NUMBER}
                            git config --global user.email "krin@ra.el"
                            git config --global user.name "Krin"
                            git merge ${incoming_branch}''')

                        // Some debug code
                        shWithErr('ls -lash')
                    } catch (Exception e) {
                        sendError("Repo error", e.toString())
                    }
                }
            }
        }
        stage('format') {
            agent {
                docker {
                    image "${DOCKER_IMAGE}"
                    args "${DOCKER_ARGS}"
                    registryUrl "${HTTP}://${DOCKER_REPO_URL}"
                    registryCredentialsId "${DOCKER_REPO_CREDENTIALS_ID}"
                    reuseNode true
                }
            }
            steps {
                echo 'Checking formatting of libraries..'
                // This is the business end.
                script {
                    try {
                        shWithErr('''
                            cd aiomni-core
                            cargo fmt --all --check'''
                        )
                    } catch (Exception e) {
                        sendError("Format error", e.toString())
                    }
                }
            }
        }
        stage('lint') {
            agent {
                docker {
                    image "${DOCKER_IMAGE}"
                    args "${DOCKER_ARGS}"
                    registryUrl "${HTTP}://${DOCKER_REPO_URL}"
                    registryCredentialsId "${DOCKER_REPO_CREDENTIALS_ID}"
                    reuseNode true
                }
            }
            steps {
                echo 'Linting/building libraries..'
                script {
                    try {
                        // Clippy should be run for all libraries in the collection
                        // and should be set to deny, to prevent warnings from accumulating.
                        shWithErr('''
                            cd aiomni-core
                            cargo clippy --all-targets -- -Dwarnings'''
                        )
                    } catch (Exception e) {
                        sendError("Lint error", e.toString())
                    }
                }
            }
        }
        // Currently we only have a build of the tests here.
        // In the future we should run proper builds for functional tests,
        // but probably after the `tests-run` stage.
        stage('tests') {
            agent {
                docker {
                    image "${DOCKER_IMAGE}"
                    args "${DOCKER_ARGS}"
                    registryUrl "${HTTP}://${DOCKER_REPO_URL}"
                    registryCredentialsId "${DOCKER_REPO_CREDENTIALS_ID}"
                    reuseNode true
                }
            }
            steps {
                script {
                    try {
                        echo 'Building library tests.. And run them!'
                        shWithErr('service postgresql start')
                        shWithErr('''
                            cd aiomni-core
                            AIOMNI_JENKINS_BUILD=true cargo test'''
                        )
                    } catch (Exception e) {
                        sendError("Tests error", e.toString())
                    }
                }
            }
        }
        // Run the internal/unit-like tests.
        // NB/TODO: Functional testing should be run only after these succeed.
    }
    // NB: The post stage is always run. The `always` step comes first.
    //     The `cleanup` step comes last. All the others are conditional
    //     On what happens in the pipeline.
    post {
        always {
            echo '`aiomni-core` pipeline finished. Running cleanup.'
        }
        aborted {
            // We do need to change the PR state on failure, since we unapproved it in the first step.
            // echo "MR no. ${mr_id} ABORTED"
            echo "`aiomni-core` pipeline aborted. Cannot merge."
            // We notify the PR that it has failed (or aborted), and link to the build.
            this.callWithCreds(
                JENKINS,
                '''
                    curl --request POST \
                    --header 'Accept: application/json;charset=UTF-8' \
                    --header 'Content-Type: application/json' \
                    --data '{"text":"__For:__ '${incoming_branch}':'${incoming_hash}'\\n\\n⚠️ __Aborted build. Will not merge.__\\n\\n___\\n__Build URL:__ '${BUILD_URL}'"}' \
                    --user ${UN}:${PW} \
                    --url "${HTTP}://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests/${mr_id}/comments"'''
            )
            this.sendBBBStatus('FAILED', "Build was aborted")
        }
        failure {
            echo "FAILED"
            script {
                if (this.needsWorkFromNotJenkins() == false) {
                // We do need to change the PR state on failure, since we unapproved it in the first step.
                    echo "MR no. ${mr_id} FAILED"
                    echo "`ai-omni-core` pipeline failed. Cannot merge."
                    // We notify the PR that it has failed, and link to the build.
                    String e = sh(script: "cat ${FINAL_ERROR_FILE}", returnStdout: true).trim()
                    e = e.replaceAll('\n', '::::::n')
                    e = e.replaceAll("\\\\", '')
                    e = e.replaceAll("'", "")
                    e = e.replaceAll('"', '')
                    e = e.replaceAll('::::::n', '\\\\n')
                    withCredentials([usernamePassword(
                        credentialsId: JENKINS,
                        usernameVariable: 'UN',
                        passwordVariable: 'PW'
                    )]) {
                        sh """
                            curl --request POST \
                            --header 'Accept: application/json;charset=UTF-8' \
                            --header 'Content-Type: application/json' \
                            --data '{"text":"__For:__ ${incoming_branch}:${incoming_hash}\\n\\n⚠️ __A build has failed with the following error:__\\n___\\n\\n```\\"${e}\\"\\n```\\n___\\n__Build URL:__ ${BUILD_URL}" }\' \
                            --user "${UN}:${PW}" \
                            --url "${HTTP}://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests/${mr_id}/comments"
                        """
                    }
                    this.sendBBBStatus('FAILED', e)
                }
            }
        }
        success {
            echo "MR no. ${mr_id} SUCCESS"
                // NB: The exact message may vary depending on bitbucket version.
                //     Also the address should use variables from the incoming webhook,
                //     but this a proof of concept.
                this.callWithCreds(
                    JENKINS,
                    '''
                        curl --request PUT \
                        --header 'Accept: application/json;charset=UTF-8' \
                        --header 'Content-Type: application/json' \
                        --data '{"status":"APPROVED"}' \
                        --user ${UN}:${PW} \
                        --url "${HTTP}://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests/${mr_id}/participants/jenkins"'''
                )
                this.sendBBBStatus('SUCCESSFUL', 'Build was successful')
            echo "`aiomni-core` test pipeline successful. Jenkins allows merge."
        }
        cleanup {
            echo "Cleaning up pipeline.."
            script {
                docker.image("${DOCKER_IMAGE}").inside(
                    "${DOCKER_ARGS}"
                ) {
                    // This script (and its maintenance) can be simplified by cleaning everything
                    // and then we do not have to keep track of which packages need to be cleaned.
                    // First clean, then delete.
                    sh '''
                        cd aiomni-core
                        cargo clean -p ai-omni-core -p chat -p db -p db_derive -p llm -p llm-client -p queue
                    '''
                    // Remove the merge branch. We should first reset, just we cannot checkout on
                    // a branch that is not clean (it can be dirty if we fail due to a merge conflict).
                    sh '''
                        cd aiomni-core
                        git reset --hard
                        git checkout master
                        git branch -D ${receiving_branch}_${incoming_branch}_test_merge_${BUILD_NUMBER}
                    '''
                }
            }
            echo "..Cleaned up pipeline."
        }
    }
}
