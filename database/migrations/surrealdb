DEFINE TABLE IF NOT EXISTS artist SCHEMAFULL
	PERMISSIONS
		FOR select, create, update FULL
		FOR delete WHERE ADMIN = $auth.ROLE;

DEFINE FIELD IF NOT EXISTS fullname ON TABLE artist TYPE string
	ASSERT string::len($value) <= 50 AND string::len($value) >= 4;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE artist TYPE datetime
    DEFAULT time::now()
    READONLY
    PERMISSIONS
        FOR create WHERE $value < time::now() + 1d AND $value > time::now() - 1d;
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE artist TYPE datetime
    DEFAULT time::now()
    PERMISSIONS
        FOR create, update WHERE $value < time::now() + 1d AND $value > time::now() - 1d;

DEFINE TABLE IF NOT EXISTS song SCHEMAFULL
	PERMISSIONS
		FOR select, create, update FULL
		FOR delete WHERE ADMIN = $auth.ROLE;

DEFINE FIELD IF NOT EXISTS title ON TABLE song TYPE string
	ASSERT string::len($value) <= 100 AND string::len($value) >= 4;
DEFINE FIELD IF NOT EXISTS lyrics ON TABLE song TYPE string
	ASSERT string::len($value) <= 6000 AND string::len($value) >= 100;
DEFINE FIELD IF NOT EXISTS view_count ON TABLE song TYPE number
    DEFAULT 1
	ASSERT $value > 0;
DEFINE FIELD IF NOT EXISTS published ON TABLE song TYPE bool
    DEFAULT true
	PERMISSIONS
		FOR update WHERE ADMIN = $auth.ROLE;
DEFINE FIELD IF NOT EXISTS artists ON TABLE song TYPE array
    ASSERT array::len($value) <=75;
DEFINE FIELD IF NOT EXISTS artists.* ON TABLE song TYPE record<artist>;
DEFINE FIELD IF NOT EXISTS created_at ON TABLE song TYPE datetime
    DEFAULT time::now()
    READONLY
    PERMISSIONS
        FOR create WHERE $value < time::now() + 1d AND $value > time::now() - 1d;
DEFINE FIELD IF NOT EXISTS updated_at ON TABLE song TYPE datetime
    DEFAULT time::now()
    PERMISSIONS
        FOR create, update WHERE $value < time::now() + 1d AND $value > time::now() - 1d;

DEFINE ANALYZER IF NOT EXISTS punct_lower_ascii TOKENIZERS PUNCT FILTERS LOWERCASE,ASCII;

DEFINE INDEX IF NOT EXISTS artist_fullname ON artist FIELDS fullname SEARCH ANALYZER punct_lower_ascii BM25(1.2,0.75) DOC_IDS_ORDER 100 DOC_LENGTHS_ORDER 100 POSTINGS_ORDER 100 TERMS_ORDER 100 DOC_IDS_CACHE 100 DOC_LENGTHS_CACHE 100 POSTINGS_CACHE 100 TERMS_CACHE 100 HIGHLIGHTS;

DEFINE FUNCTION IF NOT EXISTS fn::get_songs($page: number, $limit: option<number>) {
    LET $limite = IF type::is::none($limit) { RETURN 25; } ELSE { RETURN $limit; };
	RETURN SELECT * FROM song WHERE published = true ORDER BY created_at DESC LIMIT $limite START $page;
};

DEFINE FUNCTION IF NOT EXISTS fn::get_songs_fetch_artist($page: number, $limit: option<number>) {
    LET $limite = IF type::is::none($limit) { RETURN 25; } ELSE { RETURN $limit; };
	RETURN SELECT * FROM song WHERE published = true ORDER BY created_at DESC LIMIT $limite START $page FETCH artists;
};

DEFINE FUNCTION IF NOT EXISTS fn::get_songs_fetch_artist_by_view($page: number, $limit: option<number>) {
    LET $limite = IF type::is::none($limit) { RETURN 25; } ELSE { RETURN $limit; };
	RETURN SELECT * FROM song WHERE published = true ORDER BY view_count DESC LIMIT $limite START $page FETCH artists;
};

DEFINE FUNCTION IF NOT EXISTS fn::get_song_fetch_artist($id: record<song>) {
    UPDATE song SET view_count += 1 WHERE id = $id;
    RETURN SELECT * FROM song WHERE published = true AND id = $id FETCH artists;
};

DEFINE FUNCTION IF NOT EXISTS fn::get_unknows_artists($fullnames: string) {
    LET $fullnames = string::split($fullnames, ',');
    LET $artists = SELECT fullname FROM artist WHERE fullname INSIDE $fullnames;
    LET $fullnames_know = function() {
        return await surrealdb.value("$artists[*].fullname");
    };
    RETURN array::difference($fullnames, $fullnames_know);
};

DEFINE FUNCTION IF NOT EXISTS fn::create_artists($fullnames: array<string>) {
    LET $fullnames = $fullnames;
    RETURN function() {
        const fullnames = await surrealdb.value("$fullnames");
        const created_a = [];
        for (let fullname of fullnames) {
            if (fullname.length > 0) {
                const new_a = await surrealdb.query("CREATE artist CONTENT {'fullname': $f}", {
                    'f': fullname,
                });
                created_a.push(new_a[0]);
            }
        }
        return created_a;
    }
};

DEFINE FUNCTION IF NOT EXISTS fn::artists_to_ids($artists: array) {
    LET $artists = $artists;
    RETURN function() {
        return await surrealdb.value("$artists[*].id");
    };
};

DEFINE FUNCTION IF NOT EXISTS fn::create_song($title: string, $lyrics: string, $fullnames: string) {
    LET $title = $title;
    LET $lyrics = $lyrics;
    LET $fullnames = $fullnames;

    LET $knows_artists = SELECT * FROM artist WHERE fullname INSIDE string::split($fullnames, ',');
    LET $unknows_artists = fn::get_unknows_artists($fullnames);
    LET $new_artists = fn::create_artists($unknows_artists);
    LET $artists = array::concat($knows_artists, $new_artists);
    LET $artists = fn::artists_to_ids($artists);

    LET $new_song = CREATE song CONTENT {
        'title': $title,
        'lyrics': $lyrics,
        'artists': $artists,
    };

    RETURN $new_song[0];
};