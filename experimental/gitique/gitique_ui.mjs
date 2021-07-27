// Do something here

import { getUser, getPulls, getMerged } from './github.mjs'

// For <settings-ui />
import './settings_ui.mjs'

const MAX_PRS_PER_SECTION = 15;
const RECENCY_LIMIT = 86400 * 10 * 1000; // 10 days

this.state = {
  pulls: [],
  reviews: [],
  myMerged: [],
  otherMerged: [],
  settingsVisible: true,
}

const userPromise = getUser()

getPulls().then(async (prs) => {
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
})

getMerged().then(async (merged) => {
  let user = await userPromise;
  const myMerged = merged.filter((p) => p.user.login == user.login).slice(0,MAX_PRS_PER_SECTION)
  const otherMerged = merged.filter((p) => p.user.login != user.login).slice(0,MAX_PRS_PER_SECTION)

  this.setState({
    myMerged, otherMerged
  })
})
