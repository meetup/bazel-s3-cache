# bazel s3 cache [![Build Status](https://travis-ci.com/meetup/bazel-s3-cache.svg?branch=master)](https://travis-ci.com/meetup/bazel-s3-cache) [![Coverage Status](https://coveralls.io/repos/github/meetup/bazel-s3-cache/badge.svg?branch=master)](https://coveralls.io/github/meetup/bazel-s3-cache?branch=master)

> a serverless implementation for a bazel build cache

## 🤔 about

Bazel is an input output machine. Bazel's exploits the ability to track consistent sets of inputs
and their outputs to avoid rebuilding what input combinations have already been built using a [remote caching](https://docs.bazel.build/versions/master/remote-caching.html) server. This repository
contains a serverless implementation of that protocol that uses AWS API gateway triggered lambda and s3 storage
backend.

> 💡 Note API Gateway's [constraints](https://docs.aws.amazon.com/apigateway/latest/developerguide/limits.html) when evaluating this implementation. For instance, API Gateway sets a hard `10MB` limit on HTTP PUT requests.

## 👩‍🏭 development



This is a [rustlang](https://www.rust-lang.org/en-US/) application.
Go grab yourself a copy with [rustup](https://rustup.rs/).

## 🚀 deployment

This is a rust application deployed using ⚡ [serverless](https://serverless.com/) ⚡.

> 💡 To install serverless, run `make dependencies`

This lambda is configured through its environment variables.

| Name          | Description                                      |
|---------------|--------------------------------------------------|
| `USERNAME`    | basic auth username                              |
| `PASSWORD`    | basic auth password                              |

Run `AWS_PROFILE=prod make deploy` to deploy.