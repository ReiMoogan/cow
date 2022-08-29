CREATE PROCEDURE [Ranking].[ProvideExp] 
	@serverid decimal(20, 0),
	@userid decimal(20, 0)
AS
BEGIN
	SET NOCOUNT ON;
	SET XACT_ABORT ON;

	DECLARE @levelup int;
	SET @levelup = -1;

	BEGIN TRAN

	EXEC [Ranking].[AddUser] @serverid, @userid;

	/** Must be after the exec, otherwise timeout may not be initialized **/
	DECLARE @timeout int;
	SET @timeout = (SELECT [timeout] FROM [Ranking].[Server] WHERE id = @serverid);

	UPDATE Cow.Ranking.Level SET last_message = SYSDATETIME(), xp = xp + (FLOOR(5 * RAND()) + 3) WHERE server_id = @serverid AND [user_id] = @userid AND SYSDATETIME() > DATEADD(MILLISECOND, @timeout, last_message);

	DECLARE @curlevel int;
	DECLARE @curxp int;
	SELECT @curlevel = [level], @curxp = xp FROM [Ranking].[Level] WHERE [server_id] = @serverid AND [user_id] = @userid;

	DECLARE @faketable TABLE (xp int);
	DECLARE @xpnext int;
	INSERT INTO @faketable EXEC [Ranking].[CalculateLevel] @level = @curlevel, @xp = @xpnext OUTPUT;
	
	IF @curxp >= @xpnext
	BEGIN
		UPDATE Cow.Ranking.Level SET xp = @curxp - @xpnext, [level] = [level] + 1 WHERE server_id = @serverid AND [user_id] = @userid;
		SET @levelup = @curlevel + 1;
	END


	COMMIT

	/** Return whether they levelled up (and to what level), and if so, their old rank and new rank. **/
	SELECT @levelup, (SELECT TOP 1 role_id FROM [Ranking].[Role] WHERE server_id = @serverid AND min_level < @levelup ORDER BY min_level DESC), (SELECT TOP 1 role_id FROM [Ranking].[Role] WHERE server_id = @serverid AND min_level = @levelup);

	RETURN;
END