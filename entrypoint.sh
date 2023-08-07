#!/bin/sh

echo MEOWCOV_SOURCE_PREFIX = $MEOWCOV_SOURCE_PREFIX
echo MEOWCOV_REPO_NAME = $MEOWCOV_REPO_NAME
echo MEOWCOV_COMMIT_ID = $MEOWCOV_COMMIT_ID
echo MEOWCOV_NEW_LCOV = $MEOWCOV_NEW_LCOV
echo MEOWCOV_OLD_LCOV = $MEOWCOV_OLD_LCOV
echo MEOWCOV_PR_NUMBER = $MEOWCOV_PR_NUMBER

if [ -z $MEOWCOV_PR_NUMBER ]
then
    meow-coverage --source-prefix $MEOWCOV_SOURCE_PREFIX --repo-name $MEOWCOV_REPO_NAME --commit-id $MEOWCOV_COMMIT_ID --github-token $MEOWCOV_GITHUB_TOKEN --new-lcov-file $MEOWCOV_NEW_LCOV push
elif [ -z $MEOWCOV_OLD_LCOV ]
then
    meow-coverage --source-prefix $MEOWCOV_SOURCE_PREFIX --repo-name $MEOWCOV_REPO_NAME --commit-id $MEOWCOV_COMMIT_ID --github-token $MEOWCOV_GITHUB_TOKEN --new-lcov-file $MEOWCOV_NEW_LCOV pull-request --pr-number $MEOWCOV_PR_NUMBER
else
    meow-coverage --source-prefix $MEOWCOV_SOURCE_PREFIX --repo-name $MEOWCOV_REPO_NAME --commit-id $MEOWCOV_COMMIT_ID --github-token $MEOWCOV_GITHUB_TOKEN --new-lcov-file $MEOWCOV_NEW_LCOV pull-request --pr-number $MEOWCOV_PR_NUMBER --old-lcov-file $MEOWCOV_OLD_LCOV
fi
