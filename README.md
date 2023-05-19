# K-Downloader

A tool to offline backup your [Kodansha](https://kodansha.us) library.

## Installation

Currently we don't distribute pre-compiled binaries, so the best way to install this k-downloader is with cargo. 
`cargo install --git https://github.com/BastianInuk/k-download.git`

Run this command, and cargo should automatically install the tool for you.

## Usage

After installing, the first time you run the program with `k-download`, it'll prompt you to login, your login credentials will then be saved onto your system in case it'll be needed again in the future.

Once you're successfully logged in, you're presented with a view that has your library, a download queue and the path where your books will be downloaded. By design, k-downloader won't index your chapters, so you cannot download those as of right now. 

To select the volumes you want to download, you press the `l` key to highlight the library view, then you go up and down the library view with either j and k or the arrow keys. To select the volumes you press the space bar or a key. 

Once you've selected your volumes, it's time to download, if you have not run the program yet, you have to select a destination for you books, you do this by pressing the `f` key, browse to your desired path or folder and press the enter key.

When you have a queue *and* a destination, it's time to download your volumes, you do that by pressing the D key. Currently the tool will download three volumes simultaneously with ten pages each, this is done so the Kodansha servers won't rate limit the tool.

## Contributions

Contributions are always welcome. If you have any features you want or bug fixes, please file PR's like you would any other open source project.
