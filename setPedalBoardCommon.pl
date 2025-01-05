#!/usr/bin/perl -w
use strict;
use IO::Socket::INET;

## Set up the simulators and jack pipes between them.  Read data
## written by `getPedalBoardCommands.pl`

my $pedal_dir = $ENV{PEDAL_DIR};
print STDERR " * Initialise the pedal definitions";
-d $pedal_dir or die "'$pedal_dir' not a directory";
my $initialise_fn = "$pedal_dir/Initialise";
#-r $initialise_fn or die "$!: '$initialise_fn'";
open(my $fh, $initialise_fn) or die "$!: $initialise_fn";
my @config = map{chomp ; $_} <$fh>;
my @add = grep{s/^mh //} map{chomp ; $_} grep {/^mh add/} @config; 
my @param = grep{s/^mh //} map{chomp ; $_} grep {/^mh param_set /} @config; 
my @jack_initial = grep{s/^jack //} map{chomp ; $_} grep {/^jack /} @config; 
close $fh or die $!;

## Set up the effects, and the parameters
my @mh_commands = ();
push(@mh_commands, @add);
push(@mh_commands, @param);
&mod_host(\@mh_commands);
foreach my $jcmd ( @jack_initial ) {
    &handle_jack( "connect $jcmd" );
}

## `handle_mh_cmd` and `mod_host` set up the LV2 simulators.
## `mod_host` is passed an array of commands to send to `mod-host`
sub handle_mh_cmd( $$ ) {
    my ($sock, $cmd) = @_;
    warn "handle_mh_cmd(SOCK, $cmd)\n";
    print $sock "$cmd\n";

    my $result = '';
    my $r = &fhbits($sock);
    my $res = '';
    my ($nfound, $timeleft) =
	select(my $rout = $r, my $wout = undef, my $eout = undef,
	       0.5);
    if($nfound){
	my $os = 0;
	while(my $c = read($sock, $res, 1)){
	    if($c != 1 or
	       ord($res) == 0){
		last;
	    }
	    $result .=  $res;
	}
    }
    # warn "handle_mh_cmd: \$result $result\n";
    if($result =~ /resp ([\-0-9]+)/){
	# If status is a negative number an error has
	# occurred. The table below shows the number of each
	# error.
	
	# status 	error
	# -1 	ERR_INSTANCE_INVALID
	# -2 	ERR_INSTANCE_ALREADY_EXISTS
	# -3 	ERR_INSTANCE_NON_EXISTS
	# -4 	ERR_INSTANCE_UNLICENSED
	# -101 	ERR_LV2_INVALID_URI
	# -102 	ERR_LV2_INSTANTIATION
	# -103 	ERR_LV2_INVALID_PARAM_SYMBOL
	# -104 	ERR_LV2_INVALID_PRESET_URI
	# -105 	ERR_LV2_CANT_LOAD_STATE
	# -201 	ERR_JACK_CLIENT_CREATION
	# -202 	ERR_JACK_CLIENT_ACTIVATION
	# -203 	ERR_JACK_CLIENT_DEACTIVATION
	# -204 	ERR_JACK_PORT_REGISTER
	# -205 	ERR_JACK_PORT_CONNECTION
	# -206 	ERR_JACK_PORT_DISCONNECTION
	# -301 	ERR_ASSIGNMENT_ALREADY_EXISTS
	# -302 	ERR_ASSIGNMENT_INVALID_OP
	# -303 	ERR_ASSIGNMENT_LIST_FULL
	# -304 	ERR_ASSIGNMENT_FAILED
	# -401 	ERR_CONTROL_CHAIN_UNAVAILABLE
	# -402 	ERR_LINK_UNAVAILABLE
	# -901 	ERR_MEMORY_ALLOCATION
	# -902 	ERR_INVALID_OPERATION

	#     A status zero or positive means that the command was
	#     executed successfully. In case of the add command,
	#     the status returned is the instance number. The
	#     value field currently only exists for the param_get
	#     command.
	if($1 < 0 and $1 != -2){
	    print  STDERR ">> FAIL $cmd >>  $result\n";
	}else{
	    # print  STDERR ">> SUCCESS $cmd >>  $result\n";
	    return 1;
	}
    }else{
	print STDERR ">> Unexpected result: $result ";
    }
    return 0;
}    
sub mod_host( $ ){
    my $cmds = shift or die;
    my @cmds = @$cmds;

    my $remote = "localhost";

    my $mod_host_port_p = $ENV{MODHOST_PORT};
    my $sock = new IO::Socket::INET( PeerAddr => 'localhost',
				     PeerPort => $mod_host_port_p, 
				     Proto => 'tcp') or
	die "$!: Failed to connect to mod-host localhost:$mod_host_port_p ".
	"lsof -i :$mod_host_port_p: ".`lsof -i :$mod_host_port_p` . ' '; 

    ## Debugging why some effects randomly fail to be added
    my $failed = 0;
    
    foreach my $cmd (@cmds){
	# warn "Process: \$cmd($cmd) \n";
	# print STDERR  "mod-host: $cmd\n";
	if(!$failed){
	    &handle_mh_cmd($sock, $cmd);
	}
	## If command was an `add` check the effects got added
	if($cmd =~ /^add.+\s(\d+)/){
	    # print STDERR "$cmd\n";
	    # warn "Before jack_lsp\n";
	    my $jack = grep{/effect_$1/} `jack_lsp`;
	    # warn "after jack_lsp\n";
	    if(!$jack){
		print STDERR "$cmd: effect_$1 failed\n";
		$failed = 1;
	    }else{
		$failed = 0;
		# print STDERR "Got effect_$1\n";
	    }
	}
    }
}


## Handle a connecet or disconnect jack command: When there are no
## spaces in MIDI device names
sub handle_jack( $ ){
	 my $str = shift or die;
	 my ($cmd, $lhs, $rhs) = split(/\s/, $str);;
	 return &handle_jack_3($cmd, $lhs, $rhs);
}

## Handle a connecet or disconnect jack command: When there spaces in
## MIDI device names
sub handle_jack_3( $$$ ){
    ## Passed a Jack command execute it.  There are two: "connect"
    ## and "disconnect"
	 my ($cmd, $lhs, $rhs) = @_;

    # warn "$cmd ";
    if($cmd eq 'connect'){ ## (\S+)\s+(\S+)\s*$/){
        ## Commanded to make a connection.  Check first if it exists
        ## and there is nothing to do
        if( ! &test_jack_connection($lhs, $rhs)){
            # print STDERR "connect $1\t$2\n";
            print `jack_connect '$lhs' '$rhs'`;
        }
    }elsif($cmd =~ /^disconnect (\S+)\s+(\S+)\s*$/){
        if(  &test_jack_connection($1, $2)){
            print `jack_disconnect '$lhs' '$rhs'`;
        }
    }
}
## Check for a connetion between two ports.  
sub test_jack_connection( $$ ) { 
    my ($lhs, $rhs) = @_;
    my @jack_lsp = `jack_lsp -c`;


    my $c_lhs;
    my $c_rhs;

    my $result = 0;
    my $state = "";
    foreach my $line (@jack_lsp){
        chomp $line;
        if($line =~ /^$lhs$/){
            $state = $lhs;
            next;
        }elsif($line =~ /^\S/){
            $state = "";
            next;
        }elsif($line =~ /^\s+$rhs$/){
            if($state){
                return 1;
                exit;
            }
        }
    }
    return 0;
}

## From select(2) section of perlfunc
sub fhbits {
    my @fhlist = @_;
    my $bits = "";
    for my $fh (@fhlist) {
        vec($bits, fileno($fh), 1) = 1;
    }
    return $bits;
}
