// Do something here

import { getUser, getPulls, getMerged } from './github.mjs'

import './settings_ui.mjs'
import './pr_row.mjs'

const MAX_PRS_PER_SECTION = 15;
const RECENCY_LIMIT = 86400 * 14 * 1000; // 14 days

this.state = {
  pulls: [],
  reviews: [],
  myMerged: [],
  otherMerged: [],
  settingsVisible: true,
  hasPulls: false,
  hasReviews: false,
  hasMyMerged: false,
  hasOtherMerged: false,
}

this.stateMappers = {
  hasOtherMerged: (otherMerged) => otherMerged?.length > 0,
  hasMyMerged: (myMerged) => myMerged?.length > 0,
  hasReviews: (reviews) => reviews?.length > 0,
  hasPulls: (pulls) => pulls?.length > 0,
}

const userPromise = getUser()

const updatePulls = async (prs) => {
  let user = await userPromise;
  const pulls = prs.filter((p) => p.user.login == user.login)
  const reviews = prs.filter((p) => {
    const updatedRecently = (Date.now() - Date.parse(p.updated_at)) < RECENCY_LIMIT
    return p.user.login != user.login && updatedRecently
  }).slice(0, MAX_PRS_PER_SECTION)

  this.setState({
    pulls,
    reviews,
  })
}

// First, make a request forcing cache. Then follow it with a non-forced request
getPulls(true).then((prs) => updatePulls(prs))
  .then(() => getPulls())
  .then((prs) => updatePulls(prs))

const updateMerged = async (merged) => {
  let user = await userPromise;
  const myMerged = merged.filter((p) => p.user.login == user.login).slice(0,MAX_PRS_PER_SECTION)
  const otherMerged = merged.filter((p) => p.user.login != user.login).slice(0,MAX_PRS_PER_SECTION)

  this.setState({
    myMerged, otherMerged
  })
}

getMerged(true).then((merged) => updateMerged(merged))
  .then(() => getMerged())
  .then((merged) => updateMerged(merged))

function hideSettings() {
  this.setState({ settingsVisible: false })
}

function showSettings() {
  this.setState({ settingsVisible: true })
}
