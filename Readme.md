icfp2020-submission-system
==========================

## Description

This system's purpose was to test potential submissions,
prior to the actual submission.

> Authors Note:
> My main development machine was running Windows 
> ,and I ran in to [this bug](https://gitlab.haskell.org/ghc/ghc/-/issues/17926) (or one with the same effect).
> Therefor this was rather useful as I couldn't compile on my main development machine as 
> As ghc 8.8.3 is the only version provided in the Docker container.

## To mimic the real submission system:
  * Build Container using pristine Docker image, 
    by replacing a present Dockerfile one or creating one if absent
  * No Network Access during Container Build (build.sh)
  * Run Container with default Entrypoint (run.sh) and two arguments
  
## The differences:
  * tests master branch in addition to submission and submissions/* branches
  * run.sh has Network Acceess
  * the arguments to run.sh do not resemble a valid server or playerKey
  * additional test.sh run after run.sh

# See Also

* [ICFP2020 Submission of Team Hastronaut](https://github.com/cau-placc/icfpcontest2020)