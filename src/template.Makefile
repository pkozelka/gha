REPO ?= {{repo}}
REF ?= {{reference}}
JOB_DIR ?=

# Authentication
GITHUB_TOKEN ?=
CURL_AUTH ?=-H "Authorization: Bearer $(GITHUB_TOKEN)"
# or, in case you prefer ~/.netrc:
#CURL_AUTH=--netrc

__COMMA__ := ,
define WORKFLOW_DISPATCH
	mkdir -p "$(JOB_DIR)"
	echo '{"ref":"$(REF)","inputs":{$(subst ++|++,$(__COMMA__),$2)}}' > $(JOB_DIR)/init-request.json
	echo '$1' > $(JOB_DIR)/workflow.txt
	curl --fail -vSL 'https://api.github.com/repos/$(REPO)/actions/workflows/$1/dispatches' \
	$(CURL_AUTH) \
	-H "X-GitHub-Api-Version: 2022-11-28" \
	-H "Accept: application/vnd.github+json" \
	-d @$(JOB_DIR)/init-request.json \
	-D $(JOB_DIR)/init-response-headers.json \
	> $(JOB_DIR)/init-response.json
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
