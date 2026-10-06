alias rd := release-development

release-development:
	git tag -f development
	git push origin -f development
