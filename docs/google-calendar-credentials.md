# Get a Google `client.json` for Peekoo

Peekoo's Google Calendar plugin does not ship with Google OAuth credentials. You must create your own OAuth client in Google Cloud, download the `client.json` file, and upload it in the Google Calendar panel.

## Before you start

You need:

- a Google account
- access to [Google Cloud Console](https://console.cloud.google.com/)
- the Peekoo desktop app

## 1. Create or choose a Google Cloud project

1. Open [Google Cloud Console](https://console.cloud.google.com/).
2. Click the project picker in the top bar.
3. Create a new project, or select an existing one.

Use a dedicated project if you want to keep Peekoo separate from other Google integrations.

## 2. Enable the Google Calendar API

1. In the left sidebar, open **APIs & Services -> Library**.
2. Search for **Google Calendar API**.
3. Open it, and click **Enable**.

## 3. Configure the OAuth consent screen

1. Open **APIs & Services -> OAuth consent screen**.
2. Choose **External** unless your Google Workspace requires something else.
3. Fill in the required app details:
   - **App name**: for example, `Peekoo Calendar`
   - **User support email**: your email
   - **Developer contact information**: your email
4. Save the form.

If Google asks for scopes, add the Calendar and basic profile scopes when prompted later. For personal use, the defaults are usually enough once the Calendar API is enabled.

## 4. Add yourself as a test user

If the app is still in testing mode, Google blocks sign-in for users who are not listed as test users.

1. In **OAuth consent screen**, find **Test users**.
2. Add the Google account you will use with Peekoo.
3. Save your changes.

## 5. Create a Desktop OAuth client

1. Open **APIs & Services -> Credentials**.
2. Click **Create Credentials -> OAuth client ID**.
3. For **Application type**, choose **Desktop app**.
4. Give it a name, for example `Peekoo Desktop`.
5. Click **Create**.

Peekoo uses a localhost callback during sign-in. You do not need to create a manual redirect URI for the desktop client.

## 6. Download the `client.json` file

1. After the client is created, click **Download JSON**.
2. Save the file somewhere safe.

This downloaded file is the `client.json` that Peekoo expects.

## 7. Upload the file in Peekoo

1. Open the **Google Calendar** panel in Peekoo.
2. Click **Choose File**.
3. Select the JSON file you downloaded.
4. Click **Upload**.
5. Click **Connect**.
6. Sign in to Google and approve access.

After the connection finishes, Peekoo can refresh and read your calendar events.

## Troubleshooting

### "App isn't verified"

This is expected for a personal app in testing mode. Continue only if you created the OAuth client yourself and trust the project.

### "Access blocked" or "This app is not authorized"

Usually one of these is true:

- the Google Calendar API is not enabled
- your account is not listed as a test user
- you created the wrong credential type

Use **Desktop app**, not **Web application**.

### Peekoo says the file is invalid

Make sure you uploaded the JSON file downloaded from **Credentials -> OAuth client ID**. Peekoo expects a file that contains an `installed` or `web` object with a `client_id` and `client_secret`.

## Keep the file private

Your `client.json` contains your OAuth client id and client secret. Do not commit it to git, share it publicly, or add it to this repository.
