# MyCourses calendar filter

A webservice that allows users to alias course names from Moodle calendar exports.

## Usage

Create a moodle calendar export link for all events in recent and next 60 days. [Export Calendar](https://mycourses.aalto.fi/calendar/export.php) in Aalto University MyCourses.

Set up your calendar filter setups by
```sh
$ curl -X POST https://calendar.lim.fi -d '{
  "url": "https://moodle.example.com/calendar/export_execute.php?userid=USER_ID_INT&authtoken=AUTH_TOKEN_HEX&preset_what=all&preset_time=recentupcoming",
  "renames": {
    "Course Name 1 D": "CN",
    "Introduction to Programming": "IntroP",
    "Learning Rust 101": "Rust"
  }
}'
{"filtered_url": "https://calendar.lim.fi/AUTH_TOKEN_HEX"}
```

Now you can get your filtered calendar subscription at `https://calendar.lim.fi/AUTH_TOKEN_HEX` (with your own authtoken there).

## Development

The service is written in Rust utilizing [workers-rs](https://github.com/cloudflare/workers-rs) and is deployed to Cloudflare Workers.

Running locally
```sh
$ wrangler2 dev --local
```
This exposes the endpoints to `127.0.0.1:8787`

Deploying
```sh
$ wrangler2 publish
```
