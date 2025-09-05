REPO ?= {{repo}}
REF ?= {{reference}}

# Authentication
GITHUB_TOKEN ?=
CURL_AUTH ?=-H "Authorization: Bearer $(GITHUB_TOKEN)"
# or, in case you prefer ~/.netrc:
#CURL_AUTH=--netrc
GITHUB_CURL=curl --fail -sSL $(CURL_AUTH) -H "X-GitHub-Api-Version: 2022-11-28" -H "Accept: application/vnd.github+json"
JOB_DIR := $(shell date +'/tmp/ghwd-%m%d-%H%M%S')

__COMMA__ := ,
define WORKFLOW_DISPATCH
	mkdir -p "$(JOB_DIR)"
	echo '{"ref":"$(REF)","inputs":{$(subst ++|++,$(__COMMA__),$2)}}' > $(JOB_DIR)/init-request.json
	echo '$1' > $(JOB_DIR)/workflow.txt
	$(GITHUB_CURL) 'https://api.github.com/repos/$(REPO)/actions/workflows/$1/dispatches' \
	-d @$(JOB_DIR)/init-request.json \
	-D $(JOB_DIR)/init-response-headers.json
	# JOB_DIR=$(JOB_DIR)
endef

{{#each workflows}}
{{#each targets}}
##
{{#each comment_lines}}
# {{this}}
{{/each}}
{{target}}:
{{#each required_vars}}
	test -n "$({{this}})" # requires: {{this}}
{{/each}}
	$(call WORKFLOW_DISPATCH,{{../file}},{{inputs_str}})

{{/each}}
{{/each}}
.PHONY: {{#each all_targets}}{{this}} {{/each}}

# Define the OS variable
OS := $(shell uname -s)
# Conditional variable assignment
ifeq ($(OS),Darwin)
DATE=gdate
else
DATE=date
endif

_wait-for-schedule:
	# $(JOB_DIR)
	@DATE_STR=$$(grep -i '^Date:' $(JOB_DIR)/init-response-headers.json | sed -e 's/^[^:]*: //'); \
	START_TIME=$$($(DATE) -u -Iseconds -d "$${DATE_STR}"); \
	echo "https://api.github.com/repos/$(REPO)/actions/workflows/`cat $(JOB_DIR)/workflow.txt`/runs?branch=$(REF)&created=>=$${START_TIME}" | tee $(JOB_DIR)/runs.url

	@echo '{"workflow_runs":[]}' > "$(JOB_DIR)/runs.json"
	sleep 3
	@printf "Spawning: "
	@while ! jq -e -r '.workflow_runs | sort_by(.run_started_at)[0].url' "$(JOB_DIR)/runs.json" > "$(JOB_DIR)/run.json"; do \
		printf '*'; \
		sleep 1; \
		$(GITHUB_CURL) "`cat $(JOB_DIR)/runs.url`" > $(JOB_DIR)/runs.json; \
	done
	@echo
	@jq -e -r '.workflow_runs | sort_by(.run_started_at)[0]' "$(JOB_DIR)/runs.json" > "$(JOB_DIR)/run.json"
	@printf "Scheduled: "
	@jq -e -r '.url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/run.url"
	@printf "GitHub UI: "
	@jq -e -r '.html_url' "$(JOB_DIR)/run.json"

_wait-for-completion:
	# $(JOB_DIR)
	@while jq -e -r '.status' "$(JOB_DIR)/run.json" > "$(JOB_DIR)/status.txt"; do \
		STATUS=`cat $(JOB_DIR)/status.txt`; \
		echo "`date -u -Iseconds` $$STATUS"; \
		[ "$$STATUS" == "completed" ] && break; \
		sleep 5; \
		[ "$$STATUS" == "queued" ] && sleep 10; \
		$(GITHUB_CURL) "`cat $(JOB_DIR)/run.url`" > $(JOB_DIR)/run.json; \
	done
	# $(JOB_DIR)
	@printf "Conclusion: "
	@jq -e -r '.conclusion' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/conclusion.txt"

_download_logs:
	#TODO Download logs

_download_artifacts:
	#TODO Download artifacts

await: _wait-for-schedule _wait-for-completion _download_logs _download_artifacts
	test `cat "$(JOB_DIR)/conclusion.txt"` == "success"
