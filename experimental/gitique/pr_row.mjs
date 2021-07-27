const attributes = ['review'];

import { getReviewState } from './github.mjs'
import renderDate from '../../util/js/human_date.mjs' 

function formatUsername(username) {
  if (!username) return username;

  if (username.endsWith("[bot]")) {
    return username.slice(0, -5)
  }
  return username.slice(0, 15)
}

this.state = {
  pr: { base: { repo: {} }, user: {} },
  author: "",
  reviewState: [],
  reviewers: [],
  link: "#",
  suffix: "",
}

this.stateMappers = {
  reviewState: (pr) => {
    if (!pr.number) {
      return {}
    }

    if (pr.merged_at == null) {
        getReviewState(pr).then((reviewState) => {
          this.setState({reviewState})
        })
    }
    return {}
  },
  reviewers: (reviewState) => {
    if (!reviewState || !reviewState.length) return [];

    const reviews = {}
    for(const review of reviewState)  {
      if (review.user.login == this.state.pr.user.login) continue;
      reviews[review.user.login] = mergeReviews(reviews[review.user.login], review)
    }

    const output = [];
    for (const reviewer of Object.keys(reviews)) {
      output.push({
        name: formatUsername(reviewer),
        approved: reviews[reviewer],
      })
    }
    return output
  },
  link: (pr) => pr?._links?.html?.href,
  author: (pr) => formatUsername(pr.user.login),
  suffix: (pr) => {
    if (pr.merged_at) {
      return `merged ${renderDate(new Date(pr.updated_at))}`
    } else {
      return `updated ${renderDate(new Date(pr.updated_at))}`
    }
  },
  hasReviewers: (review) => review == "true",
}

function mergeReviews(a, b) {
  if (!a) return b.state;
  if (!b) return a;

  if (a.state == "APPROVED" || b.state == "APPROVED") {
    return "APPROVED"
  } else {
    return "COMMENTED"
  }
}
