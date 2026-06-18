# AI Policy

## Preface

I (@gbj, creator and maintainer of this project) have serious concerns about the ethics and the effects of generative AI. Some of you might share those concerns. Some might not. It's always possible that I'm completely wrong. But in any case, it's clear to me that people will continue to use AI tools for programming, which means there are three possibilities:

1. Ban all outside contributions and issues
2. Ban AI-generated contributions and issues (and spend time and energy trying to identify them)
3. Allow contributions of any kind, with clear guidelines 

AI tools are perfectly capable of making small PRs that are indistinguishable from what a human would write. I don't think it's productive to incentivize secrecy around whether AI tools were used or not. Nor do I want to ban contributions entirely. Below is my attempt at a set of guidelines, based on my experience of the last 6 months or so of AI contributions.

## Guidelines

- **You are responsible for the code you submit.** If you are submitting a PR, you are asking me to review changes to the codebase. At a basic level, this means that you must understand the changes being made and their effects, and you think that they are a good idea. (If you're not sure whether they're good or not, it's okay to say so!) You should be ready to explain any part of the change. If you find yourself regularly suggesting changes and then reverting them at the first pushback, it's a good sign that you should spend more time reviewing them before making a PR.
- **Do not copy-paste AI output to communicate with other people.** You are a human writing to a human. If you want to use an AI tool to help you respond to a comment, *read its output* and write your own reply. If you cannot quite understand what it's saying, but it seems helpful to share, please preface it clearly ("Claude says...") and put it in a quote block `>` to set it off from your own comments.
- **Knowledge is cheap, wisdom is expensive.** This is very little value in generating a bunch of suggestions for *what* to do. All of the value comes from helping figure out *whether* to do them.
- **Text is cheap, understanding is expensive.** At this point, the primary bottleneck in this, and any other mature project, is the need for a responsible human being to read and think through the huge volumes of code and text being directed at it. It is much more helpful for you to spend your time, energy, and attention *filtering*, *reviewing*, and *testing* AI output from 1 PR than using AI to blast out 10 drive-by PRs.
- **Every change is a possible bug.** Some changes are clearly bugfixes (typos, etc.) Anything larger than that runs the risk of introducing untested edge cases or behavior changes. There is no value in churn for the sake of churn, or for most micro-optimizations.
- **A good issue is better than most PRs.** A good issue:
	- follows the issue template
	- provides a minimal reproducible example
	- reflects either a *bug* or a *feature request*, but usually not a micro-optimization opportunity 
	- **If you think you found a bug, open an issue first, not a PR.** If you open an issue with a minimal reproduction, and want to offer to make a PR: amazing!
- **A good PR is small and self-contained.** At this point, large changes that touch large areas of the codebase are not likely to be considered. Small changes that fix things that are obviously wrong are very welcome!
- **Respect other people's time.** If you open a PR, you are asking someone to read and think about your work. If they reply with comments or questions, please respond. If you have opened a couple of PRs and someone gives you feedback, please respond to or address that feedback before opening additional PRs. (If it's truly a drive-by and you don't want to follow up on it, please note that: "This is just here in case it's useful to you, feel free to use or ignore.")
- Finally: **AI is a tool, not an excuse.** We have a Code of Conduct and a [`CONTRIBUTING.md`](./CONTRIBUTING.md) that describes values and processes for making PRs. There are basic norms that have governed collaborative open source projects for decades (and all collaborative work for much longer). The fact that you now have a very powerful way to generate text does not change the underlying expectations about what you do with that text.
