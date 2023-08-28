#!/bin/bash

echo MEOWCOV_SOURCE_PREFIX = $MEOWCOV_SOURCE_PREFIX
echo MEOWCOV_REPO_NAME = $MEOWCOV_REPO_NAME
echo MEOWCOV_COMMIT_ID = $MEOWCOV_COMMIT_ID
echo MEOWCOV_NEW_LCOV = $MEOWCOV_NEW_LCOV
echo MEOWCOV_OLD_LCOV = $MEOWCOV_OLD_LCOV
echo MEOWCOV_PR_NUMBER = $MEOWCOV_PR_NUMBER
echo MEOWCOV_BRANCH = $MEOWCOV_BRANCH
echo MEOWCOV_COVERAGE_REPO = $MEOWCOV_COVERAGE_REPO
echo MEOWCOV_COVERAGE_TEAM = $MEOWCOV_COVERAGE_TEAM
echo MEOWCOV_REBUILD_RECORDS = $MEOWCOV_REBUILD_RECORDS


# If a value is provided for the records directory, assume we want to rebuild records
if [[ -n $MEOWCOV_REBUILD_RECORDS ]]
then
    meow-coverage --github-token $MEOWCOV_GITHUB_TOKEN --repo-name $MEOWCOV_REPO_NAME tracking --coverage-repo-name $MEOWCOV_COVERAGE_REPO rebuild --records $MEOWCOV_REBUILD_RECORDS --branch $MEOWCOV_BRANCH
elif [[ -n $MEOWCOV_COVERAGE_REPO ]] || [[ -n $MEOWCOV_COVERAGE_TEAM ]] # Otherwise if both the coverage repo and coverage team are provided assume we are running on a commit where the report will be gathered
then
    meow-coverage --github-token $MEOWCOV_GITHUB_TOKEN --repo-name $MEOWCOV_REPO_NAME coverage-run --source-prefix $MEOWCOV_SOURCE_PREFIX --commit-id $MEOWCOV_COMMIT_ID --new-lcov-file $MEOWCOV_NEW_LCOV push-with-report --coverage-repo $MEOWCOV_COVERAGE_REPO --coverage-team $MEOWCOV_COVERAGE_TEAM --branch $MEOWCOV_BRANCH
elif [[ -z $MEOWCOV_PR_NUMBER ]] # Otherwise if no PR number is specified assume we will just run on a commit without gathering the report
then
    meow-coverage --github-token $MEOWCOV_GITHUB_TOKEN --repo-name $MEOWCOV_REPO_NAME coverage-run --source-prefix $MEOWCOV_SOURCE_PREFIX --commit-id $MEOWCOV_COMMIT_ID --new-lcov-file $MEOWCOV_NEW_LCOV push
elif [[ -z $MEOWCOV_OLD_LCOV ]] # Otherwise as a PR number was specified we are running on a pull request, check if we have an old LCOV file to do a comparison with
then
    meow-coverage --github-token $MEOWCOV_GITHUB_TOKEN --repo-name $MEOWCOV_REPO_NAME coverage-run --source-prefix $MEOWCOV_SOURCE_PREFIX --commit-id $MEOWCOV_COMMIT_ID --new-lcov-file $MEOWCOV_NEW_LCOV pull-request --pr-number $MEOWCOV_PR_NUMBER
else
    meow-coverage --github-token $MEOWCOV_GITHUB_TOKEN --repo-name $MEOWCOV_REPO_NAME coverage-run --source-prefix $MEOWCOV_SOURCE_PREFIX --commit-id $MEOWCOV_COMMIT_ID --new-lcov-file $MEOWCOV_NEW_LCOV pull-request --pr-number $MEOWCOV_PR_NUMBER --old-lcov-file $MEOWCOV_OLD_LCOV
fi
