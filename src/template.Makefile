REPO ?= {{repo}}
REF ?= {{reference}}

# Authentication
GITHUB_TOKEN ?=
CURL_AUTH ?=-H "Authorization: Bearer $(GITHUB_TOKEN)"
# or, in case you prefer ~/.netrc:
#CURL_AUTH=--netrc

define WORKFLOW_DISPATCH
	curl -vSL 'https://api.github.com/repos/$(REPO)/actions/workflows/$1/dispatches' \
	$(CURL_AUTH) \
	-H "X-GitHub-Api-Version: 2022-11-28" \
	-H "Accept: application/vnd.github+json" \
	-d '{"ref":"$(REF)","inputs":{$2}}'
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
