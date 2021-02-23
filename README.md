# 打打 bot

Telegram bot: [@hithit_rs_bot](https://t.me/hithit_rs_bot)

Rich text is supported in message template.
```
: /打
xxx 打了 自己！
: /^aww
xxx aww了 自己！
: /aww {}
xxx aww了 自己！
: /它 掉毛！
xxx 它 自己 掉毛！
: /{sender} {receiver} {} {0}
xxx xxx ooo ooo ooo！
: /{blabla}
key blabla not found
: /{1}
index 1 not found
: /explain
Input:
Segments {
    data: [
        Segment {
            kind: {
                BotCommand
            },
            text: "/explain",
        },
        ...
    ]
}
Rendered:
...
```

## Get Started

1. Change the const `BOT_NAME` into your bot name.
2. `cargo build --release`
3. `TELOXIDE_TOKEN=xxx ./hithit_bot`

## LICENSE

This project is licensed under the [MIT License](LICENSE.md).