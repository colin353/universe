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

export function getPulls(forceCache=false) {
  const repositories = getRepositories()
  let promises = [];

  const cachedResult = window.localStorage.getItem('pulls')
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (isRecent(value.cachedAt)) {
      return Promise.resolve(value.pulls)
    }
  }

  if (forceCache) return Promise.resolve([]);

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

    // Sort by last update
    output.sort((a, b) => {
      if (new Date(a.updated_at) > new Date(b.updated_at)) {
        return -1
      } else if (a.updated_at == b.updated_at) {
        return 0;
      } else {
        return 1;
      }
    });

    // Cache output
    window.localStorage.setItem('pulls', JSON.stringify({cachedAt: Date.now(), pulls: output}))

    return output;
  })
}

export function getMerged(forceCache=false) {
  const repositories = getRepositories()
  let promises = [];

  const cachedResult = window.localStorage.getItem('merged')
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (isRecent(value.cachedAt)) {
      return Promise.resolve(value.pulls)
    }
  }

  if (forceCache) return Promise.resolve([]);

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
    output.sort((a, b) => {
      if (new Date(a.merged_at) > new Date(b.merged_at)) {
        return -1
      } else if (a.updated_at == b.updated_at) {
        return 0;
      } else {
        return 1;
      }
    });

    // Cache output
    window.localStorage.setItem('merged', JSON.stringify({cachedAt: Date.now(), pulls: output}))

    return output;
  })
}

export function getReviewState(pr, forceCache=false) {
  const key = `reviewState${pr.number}`

  const cachedResult = window.localStorage.getItem(key)
  if (cachedResult) {
    let value = JSON.parse(cachedResult)
    if (new Date(pr.updated_at) < value.cachedAt && isRecent(value.cachedAt)) {
      return Promise.resolve(value.reviews)
    }
  }

  if (forceCache) return Promise.resolve([]);

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
