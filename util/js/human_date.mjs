const SECONDS = 1000;
const MINUTES = 60 * SECONDS;
const HOURS   = 60 * MINUTES;
const DAYS    = 24 * HOURS;
const WEEKS   = 7 * DAYS;
const MONTHS  = 30 * DAYS;
const YEARS   = 365 * DAYS;

export function humanizeInterval(interval) {
  const absInterval = Math.abs(interval)
  if (absInterval < 10*SECONDS) {
    return "just now";
  }

  let suffix = "ago";
  if (interval < 0) suffix = "from now";

  if (absInterval < 30 * SECONDS) {
    return `a few seconds ${suffix}`
  } else if (absInterval < 2 * MINUTES) {
    return `a minute ${suffix}`
  } else if (absInterval < 10 * MINUTES) {
    return `a few minutes ${suffix}`
  } else if (absInterval < 30 * MINUTES) {
    return `${5 * Math.floor(absInterval/MINUTES/5)} minutes ${suffix}`
  } else if (absInterval < 55 * MINUTES) {
    return `${15 * Math.floor(absInterval/MINUTES/15)} minutes ${suffix}`
  } else if (absInterval < 2 * HOURS) {
    return `an hour ${suffix}`
  } else if (absInterval < 20 * HOURS) {
    return `${Math.floor(absInterval/HOURS)} hours ${suffix}`
  } else if (absInterval < 45 * HOURS) {
    if (interval > 0) return "yesterday"
    else return "tomorrow"
  } else if (absInterval < 7 * DAYS) {
    return `${Math.floor(absInterval/DAYS)} days ${suffix}`
  } else if (absInterval < 14 * DAYS) {
    return `a week ${suffix}`
  } else if (absInterval < 25 * DAYS) {
    return `${Math.floor(absInterval/WEEKS)} weeks ${suffix}`
  } else if (absInterval < 45 * DAYS) {
    return `a month ${suffix}`
  } else if (absInterval < 350) {
    return `${Math.floor(absInterval/MONTHS)} months ${suffix}`
  } else if (absInterval < 730) {
    return `a year ${suffix}`
  } else {
    return `${Math.floor(absInterval/YEARS)} years ${suffix}`
  }
}

export default function humanizeDate(date) {
  return humanizeInterval(Date.now() - date)
}
