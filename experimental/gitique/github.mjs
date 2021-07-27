export function getToken() {
  return localStorage.getItem('token')
}

export function getRepositories() {
  return JSON.parse(window.localStorage.getItem('repos') || "[]")
}

export function getUser() {
  const cachedResult = window.localStorage.getItem('user')
  if (cachedResult) {
    return Promise.resolve(JSON.parse(cachedResult))
  }

  const headers = new Headers();
  const token = getToken();
  if (token !== null) {
    headers.append('Authorization', `token ${getToken()}`)
  }

  return fetch(`https://api.github.com/user`, {headers}).then((resp) => resp.json()).then((user) => {
    user.cachedAt = Date.now();
    window.localStorage.setItem('user', JSON.stringify(user))
    return user
  })
}

function isRecent(time) {
  console.log("cached at", time);
  if (!time) return false;
  return Date.now() - time < 60*1000
}

export function getPulls() {
  const repositories = getRepositories()
  let promises = [];

  const cachedResult = window.localStorage.getItem('pulls')
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (isRecent(value.cachedAt)) {
      return Promise.resolve(value.pulls)
    }
  }

  const headers = new Headers();
  const token = getToken();
  if (token !== null) {
    headers.append('Authorization', `token ${getToken()}`)
  }

  for (const repository of repositories) {
    promises.push(
       fetch(`https://api.github.com/repos/${repository}/pulls`, {headers}).then((resp) => resp.json())
    )
  }

  return Promise.all(promises).then((values) => {
    const output = values.flat();

    // Cache output
    window.localStorage.setItem('pulls', JSON.stringify({cachedAt: Date.now(), pulls: output}))

    return output;
  })
}

export function getMerged() {
  const repositories = getRepositories()
  let promises = [];

  const cachedResult = window.localStorage.getItem('merged')
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (isRecent(value.cachedAt)) {
      return Promise.resolve(value.pulls)
    }
  }

  const headers = new Headers();
  const token = getToken();
  if (token !== null) {
    headers.append('Authorization', `token ${getToken()}`)
  }

  for (const repository of repositories) {
    promises.push(
       fetch(`https://api.github.com/repos/${repository}/pulls?state=closed`, {headers}).then((resp) => resp.json())
    )
  }

  return Promise.all(promises).then((values) => {
    const output = values.flat()
      // Filter out non-merged PRs
      .filter(item => item.merged_at !== null);

    // Sort by merge date
    output.sort((a, b) => a.merged_at > b.merged_at);

    // Cache output
    window.localStorage.setItem('merged', JSON.stringify({cachedAt: Date.now(), pulls: output}))

    return output;
  })
}

export function getReviewState(pr) {
  const key = `reviewState${pr.number}`

  const cachedResult = window.localStorage.getItem(key)
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (isRecent(value.cachedAt)) {
      return Promise.resolve(value.reviews)
    }
  }

  const headers = new Headers();
  const token = getToken();
  if (token !== null) {
    headers.append('Authorization', `token ${getToken()}`)
  }

  return fetch(`https://api.github.com/repos/${pr.base.repo.full_name}/pulls/${pr.number}/reviews`, {headers}).then((resp) => resp.json()).then((reviewState) => {
    window.localStorage.setItem(key, JSON.stringify({cachedAt: Date.now(), reviews: reviewState}))
    return reviewState
  })
}
