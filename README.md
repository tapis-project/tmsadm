The tmsadm utility program allows users with access to the account running a TMS Server to issue SQL commands that directly affect the TMS Sqlite database. Specifically, administrators can LIST and DELETE records from the pubkeys, clients and delegations tables.  These minimal capabilities allow key revocation and the deletion of client and user/host identity mappings. This program fills in for administrative APIs that haven't be implemented yet.  In the future, the TMS Server will support remote administration via REST interface. See tmsadm help for details (tmsadm --help).    
