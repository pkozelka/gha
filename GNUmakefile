-include Makefile
.DEFAULT_GOAL := help
GITHUB_TOKEN=$(shell cat ~/.ssh/turbonext/CI/GCP_TOKEN.txt)
-include target/vmtools.workflow_dispatch.Makefile
#-include target/vllm-tn.workflow_dispatch.Makefile

gen:
	cargo run --package gha --bin gha -- -v gen -d $(HOME)/tmp/vllm-tn/.github/workflows -o target/vllm-tn.workflow_dispatch.Makefile

help:
	# TODO
-include .env


_download_logs:
	echo "Downloading:"

