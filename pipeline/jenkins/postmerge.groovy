// For Bitbucket 7.2, requires Generic Webhook Trigger on Jenkins.
//
// This pipeline exists to notify remaining PRs in a repository if PRs with the
// same branch(es) have had merges. This is because hook trigger conditions do not
// include (in bitbucket) a merge-push on the target branch. Thus we must trigger
// a rebuild semi-manually.
//
// This build script should be triggered only by a Merge event or if there is a push to
// the repository. This ensures that merges and pushes on intermediate PRs of complex merge
// schemes should also trigger a merge event.
//
// We then define three variables in the Generic Webhook Trigger:
//
// The trigger url is: https://tc-jenkins-03.telecontact.ru/generic-webhook-trigger/invoke?token=aiomni-core-uncheck-on-merge
//
// post: mr_id = $.pullRequest.id
// request: incoming_branch = $.pullRequest.fromRef.displayId  (or default)
// request: receiving_branch = $.pullRequest.toRef.displayId  (or default)
// request: current_branch = $.changed.ref.displayId  (or default)

import groovy.json.JsonSlurper
import groovy.transform.ToString

// We need this function here since JsonSlurper gives us something
// which does not serialize, and causes problems between asynchronous
// steps in the pipeline. Since we need the output specifically for
// sending PR ids via curl, which is asynchronous, we do this.
//
// NB: This function is written defensively, since there was a lot of
//     debugging involved.
List parseJson(rawJson, receiving_branch, incoming_branch, current_branch) {
    // The parsing is done here.
    def s = new JsonSlurper()
    Map content = (Map) s.parseText(rawJson)
    // Drop the slurper as fast as we can.
    s = null
    // Get the values (which are the PR descriptions.)
    def values = content.get("values")
    content = null
    // This is a debug step.
    def count = values.size()
    echo "length of response: ${count}"
    // Extract the PR ids: Find PR ids related to the receiving branch and save them.
    @NonCPS def List value_ids = values.findResults { pr ->
        if (pr.fromRef.displayId == "${receiving_branch}"
            || pr.toRef.displayId == "${receiving_branch}"
            || pr.fromRef.displayId == "${incoming_branch}"
            || pr.toRef.displayId == "${incoming_branch}"
            || pr.fromRef.displayId == "${current_branch}"
            || pr.toRef.displayId == "${current_branch}") {

            def int id = pr.id.toInteger()
            echo "Integer: ${id} will be pushed to PR id list."

            return id
        }
    }
    values = null
    return value_ids
}

pipeline {
    // agent any
    environment {
        BBPROJ = 'telecontact'
        BBREPO = 'aiomni-core'
        BBHOST = 'bitbucket.telecontact.ru'
        BBADDR = "${BBHOST}/scm/${BBPROJ}/${BBREPO}.git"
        JENKINS = 'jenkins_username_password'
        HTTP = 'https'
    }
    agent any
    stages {
        // We first unapprove the request, since it is being built!
        stage('Notify and needs work') {
            steps {
                script {
                    echo "unapproving PR no.${mr_id} (${incoming_branch} -> ${receiving_branch})"
                    // Get the PRs which belong to this user and are open.
                    def res = withCredentials([usernamePassword(
                        credentialsId: "${JENKINS}",
                        usernameVariable: 'UN',
                        passwordVariable: 'PW'
                    )]) {
                        def String res = sh(script:'''
                                curl --request GET \
                                --header 'Accept: application/json;charset=UTF-8' \
                                --header 'Content-Type: application/json' \
                                --user ${UN}:${PW} \
                                --url "${HTTP}://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests?state=OPEN"
                            ''',
                            returnStdout: true
                        ).trim()
                        return res
                    }
                    // Extract the PR ids of those PRs that involve the branch which
                    // was merged into.
                    def values = parseJson(res, receiving_branch, incoming_branch, current_branch)
                    echo "These are our retrieved values ${values}"
                    // For each such MR, send a "NEEDS_WORK" signal, which should trigger a new
                    // build via the `genericWebhookTriggerMerge.groovy`, if things are set up
                    // correctly.
                     withCredentials([usernamePassword(
                        credentialsId: "${JENKINS}",
                        usernameVariable: 'UN',
                        passwordVariable: 'PW'
                    )]) {
                        values.each { pr_id ->
                            echo "PR id (final check): ${pr_id}"
                            def curl_command = "curl --request PUT --header \"Accept: application/json;charset=UTF-8\" --header \"Content-Type: application/json\" --data \"{\\\"status\\\":\\\"NEEDS_WORK\\\"}\" --user ${UN}:${PW} --url \"${HTTP}://${BBHOST}/rest/api/latest/projects/${BBPROJ}/repos/${BBREPO}/pull-requests/${pr_id}/participants/jenkins\""
                            sh curl_command
                        }
                    }
                }
            }
        }
    }
}
