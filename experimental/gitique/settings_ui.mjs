// Do something here

this.state = {
  token: window.localStorage.getItem('token') || "",
  repos: JSON.parse(window.localStorage.getItem('repos') || "[]")
}

function save() {
  window.localStorage.setItem('token', this.refs.token.value)
  this.setState({
    token: this.refs.token.value,
  })
  window.localStorage.removeItem('user')
  window.localStorage.removeItem('pulls')
  window.localStorage.removeItem('merged')
  window.location.reload();
}

function remove(index) {
  this.state.repos.splice(index, 1)
  this.setState({repos: this.state.repos})
  saveRepos()
  window.location.reload();
}

function addRepo() {
  this.state.repos.push(this.refs.add_repo.value);
  this.setState({repos: this.state.repos})
  this.refs.add_repo.value = ""
  saveRepos()
  window.location.reload();
}

const saveRepos = () => {
  window.localStorage.setItem('repos', JSON.stringify(this.state.repos))
  window.localStorage.removeItem('pulls')
  window.localStorage.removeItem('merged')
}
