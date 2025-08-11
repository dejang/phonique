# Phonique
Phonique is a cross platform desktop music player meant to solve pain points that power users often encounter when they need to manage large collections of music files.

Additionally, Phonique aims to provide a module for exploring the Discogs database and preview songs right in the player, without ever having to change context. It does so by connecting to various other online music stores and matching a Discogs release to digital samples found on one of the stores. Not all vinyl releases have a digital release too but when they do, Phonique will provide a link to the store so you can purchase the songs.

Phonique aims to improve workflows that music collectors and DJs already do manually and consumes a lot of their time. It is packed with features that cater to various types of users.

<img width="1294" height="831" alt="Image" src="https://github.com/user-attachments/assets/02f85653-8019-428d-acea-f122c066edae" />

## Features (some still need implementation)
- Organize music files into (traiditional) playlists. (working)
- Tag individual music files and then create smart playlists using your tags. Smart playlists are dynamic and don't require manually adding/removing songs to them as they are generated based on the attributes you choose.
- Search and Browse the Discogs online database (working but requires authenticating with Discogs).
- Look for digital versions of vinyl releases on digital stores or stream from Youtube (Beatport and Bandcamp working).
- Connect your Google Drive or Dropbox account and stream songs directly from your cloud storage
- Analyze BPM, Song Key, soundwave, peaks, stereo width to determine the quality of the song
- Export your playlists as m3u files along with the songs in them, organized and ready to be placed on a USB so you can show up at your DJ gig ready to go and fully prepared.
- Song mixer and equalizer allows you to mix 2 songs and check how they work together. Useful when preparing a set for your gig.
- Register for a free account and collaborate with other record collectors and artists. (TBD)
- You can like a song as it is playing by pressing the heart button. See your liked songs by clicking on the Favorites sidebar section (working).

## Design
Being built on iced-rs which is part immediate mode GUI and part retained mode, the most important part of the application is state management. There is a data layer, swapping data layers should be relatively simple by simply implementing the Storage trait. The data layer is currently implemented for a SQLite database as the application is  still under heavy development and mostly used as a regular music player until basic features are complete. Sitting between the View layer and Data Layer there is a State Management layer which handles all UI critical state (playlists, songs, likes etc.).

## Final notes
This is a passion project and I am working on it in my spare time. Development is slow but learning how to build a full blown GUI application in Rust is fun. The app is currently going through a major facelift, the GUI design phase was recently completed and ready to move to development.
