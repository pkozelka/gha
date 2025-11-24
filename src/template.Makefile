REPO ?= {{repo}}
REF ?= {{reference}}

# Authentication
GITHUB_TOKEN ?=
CURL_AUTH ?=-H "Authorization: Bearer $(GITHUB_TOKEN)"
# or, in case you prefer ~/.netrc:
#CURL_AUTH=--netrc
GITHUB_CURL=curl --fail -sSL $(CURL_AUTH) -H "X-GitHub-Api-Version: 2022-11-28" -H "Accept: application/vnd.github+json"
RUNNER_TEMP ?= /tmp
JOB_DIR := $(shell date +'$(RUNNER_TEMP)/.gha-%m%d-%H%M%S-%N')

__COMMA__ := ,
__REPO__ := $(subst /,_,$(REPO))
__GHA_RECENT__ := $(RUNNER_TEMP)/.gha-recent-$(USER).$(__REPO__).txt

define WORKFLOW_DISPATCH
	mkdir -p "$(JOB_DIR)"
	echo "$(JOB_DIR)" >> $(__GHA_RECENT__)
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
{{target}}: async-{{target}} await
async-{{target}}:
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
	@DATE_STR=$$(grep -i '^Date:' $(JOB_DIR)/init-response-headers.json | sed -e 's/^[^:]*: //'); \
	START_TIME=$$($(DATE) -u -Iseconds -d "$${DATE_STR}"); \
	echo "https://api.github.com/repos/$(REPO)/actions/workflows/`cat $(JOB_DIR)/workflow.txt`/runs?branch=$(REF)&created=>=$${START_TIME}" \
	| tee $(JOB_DIR)/runs.url

	@STATUS=$(shell cat $(JOB_DIR)/status.txt 2>/dev/null); echo "STATUS: $$STATUS"; \
	if [ "$$STATUS" == "completed" ]; then true; else \
		echo '{"workflow_runs":[]}' > "$(JOB_DIR)/runs.json"; \
		sleep 3; \
		printf "Spawning: "; \
		while ! jq -e -r '.workflow_runs | sort_by(.run_started_at)[0].url' "$(JOB_DIR)/runs.json" \
		> "$(JOB_DIR)/run.json"; do \
			printf '*'; \
			sleep 1; \
			$(GITHUB_CURL) "`cat $(JOB_DIR)/runs.url`" > $(JOB_DIR)/runs.json; \
		done; \
		echo; \
		jq -e -r '.workflow_runs | sort_by(.run_started_at)[0]' "$(JOB_DIR)/runs.json" > "$(JOB_DIR)/run.json"; \
	fi
	@printf "Scheduled: "
	@jq -e -r '.url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/run.url"
	@jq -e -r '"GitHub UI: \(.html_url)"' "$(JOB_DIR)/run.json"

_wait-for-completion:
	@jq -e -r '.cancel_url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/cancel.url"
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
	# Downloading logs
	@jq -e -r '.logs_url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/logs.url"
	$(GITHUB_CURL) "`cat $(JOB_DIR)/logs.url`" > $(JOB_DIR)/logs.zip
	mkdir -p "$(JOB_DIR)/logs"
	cd "$(JOB_DIR)/logs" && unzip ../logs.zip

_download_artifacts:
	# Downloading artifacts
	@jq -e -r '.artifacts_url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/artifacts.url"
	$(GITHUB_CURL) "`cat $(JOB_DIR)/artifacts.url`" > $(JOB_DIR)/artifacts.json

_download_jobs:
	# Downloading jobs
	@jq -e -r '.jobs_url' "$(JOB_DIR)/run.json" | tee "$(JOB_DIR)/jobs.url"
	$(GITHUB_CURL) "`cat $(JOB_DIR)/jobs.url`" > $(JOB_DIR)/jobs.json

await: _wait-for-schedule _wait-for-completion _download_logs _download_artifacts _download_jobs
	test $(shell cat "$(JOB_DIR)/conclusion.txt") == "success"

await-all:
	cat "$(__GHA_RECENT__)" | while read -r DIR; do \
	  $(MAKE) -f $(firstword $(MAKEFILE_LIST)) await JOB_DIR="$$DIR"; \
	done

clean:
	cp "$(__GHA_RECENT__)" "$(__GHA_RECENT__).bak"
	cat "$(__GHA_RECENT__).bak" | while read -r DIR; do \
	  rm -rfv "$$DIR"; \
	  sed -i -e '1d' "$(__GHA_RECENT__)"; \
	done
	rm -f "$(__GHA_RECENT__).bak"
