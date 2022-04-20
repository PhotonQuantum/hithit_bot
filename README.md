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
segments {
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

You can change prefix using `HITHIT_BOT_PREFIX` environment variable or `HITHIT_BOT_PREFIX_BUILD` in compile time (default is `^`).

## Get Started

1. Declare `BOT_NAME` environment variable into your bot name (or you can set this environment variable at runtime as well).
2. Declare `BOT_SERVER` environment variable into your [custom api server](https://github.com/tdlib/telegram-bot-api) (or you can set this environment variable at runtime as well). If you don't know what this is, you can safely ignore it.  
3. `cargo build --release`
4. `TELOXIDE_TOKEN=xxx ./hithit_bot`

## LICENSE

This project is licensed under the [MIT License](LICENSE.md).